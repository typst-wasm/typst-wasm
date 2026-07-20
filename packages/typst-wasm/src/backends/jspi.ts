import {
  CompilerDisposedError,
  FileNotFoundError,
  FetchError,
  PackageFetchError,
  CompilerNotInitializedError,
  WorkerError,
} from "../errors";
import * as jspiEngineModule from "../engine/generated/jspi/engine.js";
import type { FileLoaderManager } from "../files/loaders";
import type { FontInput } from "../compiler/types";
import { getCoreModule, resolveCoreModules } from "../engine/core-modules";
import type {
  CoreModules,
  EngineCompileOptions,
  EngineCompileSuccess,
  EngineCompiler,
  EngineHost,
  EngineModule,
} from "../engine/types";

const jspiEngine: EngineModule = jspiEngineModule;

const toFetchError = (error: unknown): never => {
  if (error instanceof FileNotFoundError) throw { tag: "not-found" };
  if (error instanceof FetchError || error instanceof PackageFetchError) {
    throw { tag: "other", val: error.message };
  }
  throw {
    tag: "other",
    val: error instanceof Error ? error.message : String(error),
  };
};

export class JspiService {
  private disposed = false;
  private initPromise: Promise<void> | null = null;
  private compiler: EngineCompiler | null = null;
  private compileAsync:
    | ((options: EngineCompileOptions) => Promise<EngineCompileSuccess>)
    | null = null;

  constructor(
    private readonly fileLoaderManager: FileLoaderManager,
    private readonly coreModules: CoreModules,
  ) {}

  async init(): Promise<void> {
    this.assertNotDisposed();
    this.initPromise ??= this.initJspi();
    try {
      await this.initPromise;
    } catch (error) {
      this.initPromise = null;
      throw error;
    }
  }

  async dispose(): Promise<void> {
    this.disposed = true;
    this.compiler = null;
    this.compileAsync = null;
    this.fileLoaderManager.dispose();
  }

  async addFonts(...fonts: FontInput[]): Promise<void> {
    this.assertNotDisposed();
    const data = await Promise.all(fonts);
    this.withCompiler((compiler) => {
      for (const font of data) compiler.addFont(font);
    }, "add-fonts");
  }

  async compile(options: EngineCompileOptions): Promise<EngineCompileSuccess> {
    this.assertNotDisposed();
    const compile = this.compileAsync;
    if (!compile)
      throw new CompilerNotInitializedError("Compiler not initialized");

    try {
      return await compile(options);
    } catch (cause) {
      throw new WorkerError("JSPI command failed: compile", { cause });
    }
  }

  private async initJspi(): Promise<void> {
    const host: EngineHost = {
      fetch: async (request) => {
        try {
          return await this.fileLoaderManager.load(request);
        } catch (error) {
          return toFetchError(error);
        }
      },
      today: () => undefined,
    };

    const coreModules = await resolveCoreModules(this.coreModules);
    const root = await jspiEngine.instantiate(getCoreModule(coreModules), {
      "typst:engine/host": host,
    });
    this.compiler = new root.api.Compiler();
    this.compileAsync = async (options) =>
      await this.compiler!.compile(options);
  }

  private withCompiler<T>(
    run: (compiler: EngineCompiler) => T,
    name: string,
  ): T {
    this.assertNotDisposed();
    if (!this.compiler) {
      throw new CompilerNotInitializedError("Compiler not initialized");
    }
    try {
      return run(this.compiler);
    } catch (cause) {
      throw new WorkerError(`JSPI command failed: ${name}`, { cause });
    }
  }

  private assertNotDisposed(): void {
    if (this.disposed) {
      throw new CompilerDisposedError("Compiler has been disposed");
    }
  }
}
