import { describe, expect, it } from "vitest";
import type { EngineCompiler, EngineModule } from "../engine/types";
import { SharedMemoryCommunication } from "./protocol";
import { installTypstWorkerRuntime } from "./runtime";

type Port = {
  handler?: (data: unknown) => void;
  posted: unknown[];
};

const makePort = (): Port & {
  onMessage: (handler: (data: unknown) => void) => void;
  postMessage: (data: unknown) => void;
} => {
  const port: Port = { posted: [] };
  return Object.assign(port, {
    onMessage: (handler: (data: unknown) => void) => {
      port.handler = handler;
    },
    postMessage: (data: unknown) => port.posted.push(data),
  });
};

const compilerWithSpies = () => {
  const compiler: EngineCompiler = {
    addFont: (data) => data.byteLength.toString(),
    compile: async (options) => ({
      output: { tag: "html", val: `<p>${options.format}</p>` },
      diagnostics: [],
      dependencies: [],
    }),
  };
  return compiler;
};

const engine = (compiler: EngineCompiler): EngineModule => ({
  instantiate: async () => ({
    api: {
      Compiler: class {
        constructor() {
          return compiler;
        }
      } as unknown as new () => EngineCompiler,
    },
  }),
});

const coreModules = {
  "engine.core.wasm": {} as WebAssembly.Module,
  "engine.core2.wasm": {} as WebAssembly.Module,
  "engine.core3.wasm": {} as WebAssembly.Module,
};

const request = (kind: string, requestId: number, payload?: unknown) => ({
  kind,
  requestId,
  ...(payload === undefined ? {} : { payload }),
});

const response = async (
  port: ReturnType<typeof makePort>,
  message: unknown,
) => {
  port.handler?.(message);
  await new Promise<void>((resolve) => setTimeout(resolve, 0));
  return port.posted.at(-1) as Record<string, unknown>;
};

describe("installTypstWorkerRuntime", () => {
  it("requires core modules when configured to do so", async () => {
    const port = makePort();
    installTypstWorkerRuntime(port, async () => engine(compilerWithSpies()));

    expect(
      await response(
        port,
        request("init", 1, {
          sharedMemoryCommunication: new SharedMemoryCommunication(),
        }),
      ),
    ).toMatchObject({
      requestId: 1,
      error: {
        code: "INIT_FAILED",
        cause: { message: "Worker requires engine core modules" },
      },
    });
  });

  it("reports commands before initialization and ignores unknown messages", async () => {
    const port = makePort();
    installTypstWorkerRuntime(port, async () => engine(compilerWithSpies()));

    expect(
      await response(
        port,
        request("compile", 3, { options: { format: "html" } }),
      ),
    ).toMatchObject({
      requestId: 3,
      error: { code: "COMPILER_NOT_INITIALIZED" },
    });
    const count = port.posted.length;
    await response(port, request("unknown", 4));
    expect(port.posted).toHaveLength(count);
    await response(port, null);
    expect(port.posted).toHaveLength(count);
  });

  it("returns the init request ID and dispatches every compiler command", async () => {
    const port = makePort();
    const calls: string[] = [];
    const compiler = compilerWithSpies();
    for (const name of ["addFont", "compile"] as const) {
      const original = compiler[name] as (...args: never[]) => unknown;
      compiler[name] = ((...args: never[]) => {
        calls.push(name);
        return original(...args);
      }) as never;
    }
    installTypstWorkerRuntime(port, async () => engine(compiler));
    const communication = new SharedMemoryCommunication();
    expect(
      await response(
        port,
        request("init", 10, {
          sharedMemoryCommunication: communication,
          coreModules,
        }),
      ),
    ).toEqual({ requestId: 10, result: undefined });

    await response(
      port,
      request("add_fonts", 13, { data: [new Uint8Array([1])] }),
    );
    await response(
      port,
      request("compile", 17, { options: { format: "html" } }),
    );
    expect(calls).toEqual(["addFont", "compile"]);
  });

  it.each(["load", "instantiate", "compiler"])(
    "maps %s initialization failures",
    async (stage) => {
      const port = makePort();
      const failure = new Error(`${stage} failed`);
      const loader: () => Promise<EngineModule> =
        stage === "load"
          ? async () => {
              throw failure;
            }
          : async () => ({
              instantiate:
                stage === "instantiate"
                  ? async () => {
                      throw failure;
                    }
                  : async () => ({
                      api: {
                        Compiler: class {
                          constructor() {
                            throw failure;
                          }
                        },
                      } as never,
                    }),
            });
      installTypstWorkerRuntime(port, loader);
      const result = await response(
        port,
        request("init", 20, {
          sharedMemoryCommunication: new SharedMemoryCommunication(),
          coreModules,
        }),
      );
      expect(result).toMatchObject({
        requestId: 20,
        error: { code: "INIT_FAILED", cause: failure },
      });
    },
  );

  it("serializes nested error payloads and maps command failures", async () => {
    const port = makePort();
    const compiler = compilerWithSpies();
    compiler.compile = () => {
      throw { cause: { payload: { reason: "bad" } } };
    };
    installTypstWorkerRuntime(port, async () => engine(compiler));
    await response(
      port,
      request("init", 30, {
        sharedMemoryCommunication: new SharedMemoryCommunication(),
        coreModules,
      }),
    );
    expect(
      await response(
        port,
        request("compile", 31, { options: { format: "html" } }),
      ),
    ).toEqual({
      requestId: 31,
      error: {
        code: "COMMAND_FAILED",
        message: "Worker command failed: compile",
        cause: { payload: { reason: "bad" } },
      },
    });
  });
});
