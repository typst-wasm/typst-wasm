import {
  SharedMemoryCommunication,
  SharedMemoryCommunicationError,
  SharedMemoryCommunicationStatus,
} from "./protocol";
import type { WorkerPort } from "./port";
import {
  isMainToWorkerMessage,
  type MainToWorkerMessage,
  type WorkerToMainMessage,
} from "./messages";
import type {
  EngineCompiler,
  EngineFetchRequest,
  EngineModule,
} from "../engine/types";

type WorkerRpcErrorCode =
  | "COMPILER_NOT_INITIALIZED"
  | "INIT_FAILED"
  | "COMMAND_FAILED";

type WorkerRpcError = {
  code: WorkerRpcErrorCode;
  message: string;
  cause?: unknown;
};

type WorkerRpcResponse = Extract<WorkerToMainMessage, { requestId: number }>;

const extractErrorPayload = (error: unknown): unknown => {
  if (typeof error !== "object" || error === null) return undefined;
  if ("payload" in error) return (error as { payload?: unknown }).payload;
  if ("cause" in error)
    return extractErrorPayload((error as { cause?: unknown }).cause);
  return undefined;
};

const serializeErrorCause = (cause: unknown): unknown => {
  const payload = extractErrorPayload(cause);
  return payload === undefined ? cause : { payload };
};

class WorkerCommandError extends Error {
  constructor(
    readonly requestId: number,
    readonly code: WorkerRpcErrorCode,
    message: string,
    readonly cause?: unknown,
  ) {
    super(message);
    this.name = "WorkerCommandError";
  }
}

const toWorkerCommandError = (
  error: unknown,
  requestId: number,
): WorkerCommandError =>
  error instanceof WorkerCommandError
    ? error
    : new WorkerCommandError(
        requestId,
        "COMMAND_FAILED",
        "Unhandled worker command failure",
        error,
      );

const fetchErrorForCode = (code: SharedMemoryCommunicationError): never => {
  switch (code) {
    case SharedMemoryCommunicationError.NotFound:
      throw { tag: "not-found" };
    case SharedMemoryCommunicationError.Denied:
      throw { tag: "denied" };
    case SharedMemoryCommunicationError.Timeout:
      throw { tag: "timeout" };
    case SharedMemoryCommunicationError.Unavailable:
      throw { tag: "unavailable" };
    default:
      throw { tag: "other", val: "Worker fetch failed" };
  }
};

export const installTypstWorkerRuntime = (
  port: WorkerPort,
  loadEngine: () => Promise<EngineModule>,
): void => {
  let compiler: EngineCompiler | null = null;
  let sharedMemoryCommunication: SharedMemoryCommunication | null = null;

  const hostFetch = (request: EngineFetchRequest) => {
    if (!sharedMemoryCommunication) {
      throw new Error("Communication buffer not initialized");
    }

    for (let attempt = 0; attempt < 3; attempt += 1) {
      sharedMemoryCommunication.setStatus(
        SharedMemoryCommunicationStatus.Pending,
      );
      port.postMessage({
        kind: "web_fetch",
        payload: { request },
      } satisfies WorkerToMainMessage);

      const changed = sharedMemoryCommunication.waitForStatusChange(
        SharedMemoryCommunicationStatus.Pending,
        30_000,
      );
      if (!changed) continue;

      if (
        sharedMemoryCommunication.getStatus() ===
        SharedMemoryCommunicationStatus.Success
      ) {
        return {
          data: new Uint8Array(sharedMemoryCommunication.getBuffer()),
        };
      }

      if (
        sharedMemoryCommunication.getStatus() ===
        SharedMemoryCommunicationStatus.Error
      ) {
        fetchErrorForCode(sharedMemoryCommunication.getError());
      }
    }

    throw { tag: "timeout" };
  };

  const successResponse = (
    requestId: number,
    result: unknown,
  ): WorkerRpcResponse => ({
    requestId,
    result,
  });

  const errorResponse = (
    requestId: number,
    code: WorkerRpcErrorCode,
    message: string,
    cause?: unknown,
  ): WorkerRpcResponse => ({
    requestId,
    error: { code, message, cause } satisfies WorkerRpcError,
  });

  const ensureCompiler = (requestId: number): EngineCompiler => {
    if (compiler) return compiler;
    throw new WorkerCommandError(
      requestId,
      "COMPILER_NOT_INITIALIZED",
      "Compiler not initialized",
    );
  };

  const runCompilerCommand = <T>(
    requestId: number,
    commandName: string,
    run: (readyCompiler: EngineCompiler) => T,
  ): T => {
    const readyCompiler = ensureCompiler(requestId);
    try {
      return run(readyCompiler);
    } catch (cause) {
      throw new WorkerCommandError(
        requestId,
        "COMMAND_FAILED",
        `Worker command failed: ${commandName}`,
        cause,
      );
    }
  };

  const handleRequest = async (
    request: MainToWorkerMessage,
  ): Promise<WorkerRpcResponse | undefined> => {
    switch (request.kind) {
      case "init": {
        sharedMemoryCommunication = SharedMemoryCommunication.hydrateObj(
          request.payload.sharedMemoryCommunication,
        );
        try {
          const engine = await loadEngine();
          const coreModules = request.payload.coreModules;
          if (!coreModules) {
            throw new Error("Worker requires engine core modules");
          }
          const root = await engine.instantiate(
            (name) => {
              const module = coreModules[name];
              if (!module) throw new Error(`Unknown core module: ${name}`);
              return module;
            },
            {
              "typst:engine/host": {
                fetch: hostFetch,
                today: () => undefined,
              },
            },
          );
          compiler = new root.api.Compiler();
        } catch (cause) {
          throw new WorkerCommandError(
            request.requestId,
            "INIT_FAILED",
            "Failed to initialize WASM worker",
            cause,
          );
        }
        return successResponse(request.requestId, undefined);
      }
      case "add_fonts":
        runCompilerCommand(request.requestId, "add-fonts", (readyCompiler) => {
          for (const font of request.payload.data) readyCompiler.addFont(font);
        });
        return successResponse(request.requestId, undefined);
      case "compile":
        return successResponse(
          request.requestId,
          await runCompilerCommand(
            request.requestId,
            "compile",
            (readyCompiler) => readyCompiler.compile(request.payload.options),
          ),
        );
    }
  };

  port.onMessage((data) => {
    if (!isMainToWorkerMessage(data)) return;

    void handleRequest(data)
      .catch((error) => {
        const commandError = toWorkerCommandError(error, data.requestId);
        return errorResponse(
          commandError.requestId,
          commandError.code,
          commandError.message,
          serializeErrorCause(commandError.cause),
        );
      })
      .then((result) => {
        // The structural message guard intentionally only validates the common
        // RPC envelope. Unknown command kinds must not produce a response.
        if (result) port.postMessage(result);
      });
  });
};
