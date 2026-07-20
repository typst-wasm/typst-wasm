import { FetchError, FileNotFoundError } from "../errors";
import type {
  FetchedFile,
  FetchRequest,
  TypstFileLoader,
} from "../compiler/types";

export type FetchImpl = (
  input: Parameters<typeof fetch>[0],
  init?: Parameters<typeof fetch>[1],
) => ReturnType<typeof fetch>;

export class MemoryFileLoader {
  private readonly files = new Map<string, Uint8Array>();

  async load(request: FetchRequest): Promise<FetchedFile | null> {
    if (request.kind !== "project") return null;
    const data = this.files.get(request.path);
    return data ? { data: new Uint8Array(data) } : null;
  }

  setSource(path: string, text: string): void {
    this.files.set(path, new TextEncoder().encode(text));
  }

  setFile(path: string, data: Uint8Array): void {
    this.files.set(path, new Uint8Array(data));
  }

  removeFile(path: string): void {
    this.files.delete(path);
  }

  clear(): void {
    this.files.clear();
  }

  listFiles(): string[] {
    return [...this.files.keys()].sort();
  }

  hasFile(path: string): boolean {
    return this.files.has(path);
  }
}

export class FileLoaderManager {
  private loaders: TypstFileLoader[];

  constructor(loaders: TypstFileLoader[]) {
    this.loaders = loaders;
  }

  dispose(): void {
    this.loaders = [];
  }

  async load(request: FetchRequest): Promise<FetchedFile> {
    for (const loader of this.loaders) {
      const result = await loader(request);
      if (result) return result;
    }
    throw new FileNotFoundError(request.path);
  }
}

export const makeFetchFileLoader =
  (fetchImpl: FetchImpl = fetch): TypstFileLoader =>
  async (request) => {
    if (request.kind === "package") return null;

    try {
      const response = await fetchImpl(request.path);
      if (!response.ok) throw new Error(`Status ${response.status}`);
      return {
        data: new Uint8Array(await response.arrayBuffer()),
        resolvedPath: response.url || undefined,
        mediaType: response.headers.get("content-type") ?? undefined,
      };
    } catch (cause) {
      throw new FetchError(request.path, cause);
    }
  };
