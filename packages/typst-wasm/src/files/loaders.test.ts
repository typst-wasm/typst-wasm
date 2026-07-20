import { describe, expect, it, vi } from "vitest";
import {
  FileLoaderManager,
  makeFetchFileLoader,
  MemoryFileLoader,
} from "./loaders";
import { makePackageFileLoader, PackageManager } from "./packages";
import type { TypstFileLoader } from "../compiler/types";
import { normalizeProjectPath } from "./paths";

describe("memory file loader", () => {
  it("owns only project files and returns defensive copies", async () => {
    const loader = new MemoryFileLoader();
    const input = new Uint8Array([1, 2]);
    loader.setFile("assets/data.bin", input);
    input[0] = 9;

    const result = await loader.load({
      path: "assets/data.bin",
      kind: "project",
    });
    expect(result?.data).toEqual(new Uint8Array([1, 2]));
    expect(loader.listFiles()).toEqual(["assets/data.bin"]);
    expect(loader.hasFile("assets/data.bin")).toBe(true);
    expect(
      await loader.load({ path: "@pkg/file", kind: "package" }),
    ).toBeNull();
  });

  it("encodes source and supports removal fallback", async () => {
    const loader = new MemoryFileLoader();
    loader.setSource("main.typ", "hello");
    expect(await loader.load({ path: "main.typ", kind: "project" })).toEqual({
      data: new TextEncoder().encode("hello"),
    });
    loader.removeFile("main.typ");
    expect(await loader.load({ path: "main.typ", kind: "project" })).toBeNull();
  });
});

describe("project paths", () => {
  it.each([
    ["main.typ", "main.typ"],
    ["./src/../main.typ", "main.typ"],
    ["src//nested.typ", "src/nested.typ"],
  ])("normalizes %s", (input, expected) => {
    expect(normalizeProjectPath(input)).toBe(expected);
  });

  it.each(["", "../main.typ", "/main.typ", "./", "a\\b.typ"])(
    "rejects %s",
    (input) => expect(() => normalizeProjectPath(input)).toThrow(),
  );
});

describe("file loader manager", () => {
  it("tries loaders in order until one handles the request", async () => {
    const first = vi.fn<TypstFileLoader>(async () => null);
    const second = vi.fn<TypstFileLoader>(async () => ({
      data: new Uint8Array([1, 2, 3]),
      resolvedPath: "/resolved/main.typ",
    }));
    const third = vi.fn<TypstFileLoader>(async () => ({
      data: new Uint8Array([9]),
    }));
    const manager = new FileLoaderManager([first, second, third]);
    const request = { path: "main.typ", kind: "project" as const };

    await expect(manager.load(request)).resolves.toEqual({
      data: new Uint8Array([1, 2, 3]),
      resolvedPath: "/resolved/main.typ",
    });
    expect(first).toHaveBeenCalledTimes(1);
    expect(second).toHaveBeenCalledTimes(1);
    expect(third).not.toHaveBeenCalled();
  });

  it("lets project loaders pass package imports to the package loader", async () => {
    const projectLoader = vi.fn<TypstFileLoader>(async (request) =>
      request.kind === "project" ? { data: new Uint8Array([1]) } : null,
    );
    const packageManager = new PackageManager();
    const getFile = vi
      .spyOn(packageManager, "getFile")
      .mockResolvedValue(new Uint8Array([4, 5]));
    const manager = new FileLoaderManager([
      projectLoader,
      makePackageFileLoader(packageManager),
    ]);
    const request = {
      path: "@preview/cetz:0.3.4/lib.typ",
      kind: "package" as const,
    };

    await expect(manager.load(request)).resolves.toEqual({
      data: new Uint8Array([4, 5]),
      resolvedPath: request.path,
    });
    expect(projectLoader).toHaveBeenCalledWith(request);
    expect(getFile).toHaveBeenCalledWith(request.path);
  });

  it("uses fetch fallback for non-package paths", async () => {
    const fetchImpl = vi.fn(
      async () =>
        new Response(new Uint8Array([7]), {
          headers: { "content-type": "application/octet-stream" },
        }),
    );
    const manager = new FileLoaderManager([makeFetchFileLoader(fetchImpl)]);

    await expect(
      manager.load({ path: "/asset.bin", kind: "project" }),
    ).resolves.toEqual({
      data: new Uint8Array([7]),
      resolvedPath: undefined,
      mediaType: "application/octet-stream",
    });
    expect(fetchImpl).toHaveBeenCalledWith("/asset.bin");
  });
});
