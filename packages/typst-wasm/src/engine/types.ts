import type {
  CompileOptions,
  CompileSuccess,
  CompileFormat,
  Diagnostic,
  FetchRequest,
  FetchedFile,
  FileKind,
  LoadedFile,
  PdfStandard,
} from "./generated/jspi/interfaces/typst-engine-types.d.ts";

export type {
  CompileFormat,
  Diagnostic,
  FetchRequest,
  FetchedFile,
  FileKind,
  LoadedFile,
  PdfStandard,
};

export type EngineFetchRequest = FetchRequest;
export type EngineCompileOptions = CompileOptions;
export type EngineCompileSuccess = CompileSuccess;
export type EngineDiagnostic = Diagnostic;

export interface EngineHost {
  fetch(request: FetchRequest): FetchedFile | Promise<FetchedFile>;
  today(
    offsetSeconds?: bigint,
  ): { year: number; month: number; day: number } | undefined;
}

export interface EngineImports {
  "typst:engine/host": EngineHost;
}

export interface EngineCompiler {
  addFont(data: Uint8Array): string;
  compile(options: CompileOptions): CompileSuccess | Promise<CompileSuccess>;
}

interface EngineRoot {
  api: { Compiler: new () => EngineCompiler };
}

export type CoreModuleName =
  | "engine.core.wasm"
  | "engine.core2.wasm"
  | "engine.core3.wasm";

export type CoreModules = Readonly<
  Record<CoreModuleName, WebAssembly.Module | Promise<WebAssembly.Module>>
>;

type EngineCoreModuleLoader = (
  name: string,
) => WebAssembly.Module | Promise<WebAssembly.Module>;

export interface EngineModule {
  instantiate(
    getCoreModule: EngineCoreModuleLoader | undefined,
    imports: EngineImports,
    instantiateCore?: (
      module: WebAssembly.Module,
      imports: WebAssembly.Imports,
    ) => WebAssembly.Instance | Promise<WebAssembly.Instance>,
  ): EngineRoot | Promise<EngineRoot>;
}
