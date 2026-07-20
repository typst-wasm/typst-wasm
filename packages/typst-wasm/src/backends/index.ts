import type { FontInput, TypstCompilerOptions } from "../compiler/types";
import type { FileLoaderManager } from "../files/loaders";
import type {
  EngineCompileOptions,
  EngineCompileSuccess,
} from "../engine/types";
import { supportsJspiBackend, supportsWorkerBackend } from "./capabilities";
import type { ResolvedLogger } from "../logging";
import { JspiService } from "./jspi";
import { WorkerService } from "./worker";

export { supportsJspiBackend, supportsWorkerBackend };

export type BackendKind = "auto" | "worker" | "jspi";
export type BackendSelection = Exclude<BackendKind, "auto"> | "none";

export type BackendService = {
  init(): Promise<void>;
  dispose(): Promise<void>;
  addFonts(...fonts: FontInput[]): Promise<void>;
  compile(options: EngineCompileOptions): Promise<EngineCompileSuccess>;
};

export interface BackendOptions {
  fileLoaderManager: FileLoaderManager;
  logger?: ResolvedLogger;
}

export const selectAutomaticBackendKind = (
  options: TypstCompilerOptions,
): BackendSelection => {
  if (options.worker && supportsWorkerBackend()) return "worker";
  if (supportsJspiBackend()) return "jspi";
  return "none";
};

export const createRuntimeBackend = (
  backend: BackendKind,
  options: BackendOptions,
  compilerOptions: TypstCompilerOptions,
): BackendService => {
  const selected =
    backend === "auto" ? selectAutomaticBackendKind(compilerOptions) : backend;
  const coreModules = compilerOptions.coreModules;

  switch (selected) {
    case "worker":
      if (!compilerOptions.worker) {
        throw new Error("Worker backend requires worker");
      }
      if (!supportsWorkerBackend()) {
        throw new Error(
          "Worker backend is unavailable: SharedArrayBuffer and growable SharedArrayBuffer support are required",
        );
      }
      return new WorkerService(options.fileLoaderManager, {
        createWorker: compilerOptions.worker,
        coreModules,
        logger: options.logger,
      });
    case "jspi":
      return new JspiService(options.fileLoaderManager, coreModules);
    case "none":
      throw new Error(
        "No compatible typst-wasm backend available. Requires Worker+SharedArrayBuffer+Atomics.wait or JSPI.",
      );
  }
};
