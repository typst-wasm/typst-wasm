import { describe, expect, it } from "vitest";
import {
  isMainToWorkerMessage,
  isRpcResponseMessage,
  isWorkerEventMessage,
} from "./messages";

describe("message guards", () => {
  it("rejects ambiguous RPC responses from untrusted worker messages", () => {
    expect(isRpcResponseMessage({ requestId: 2, result: {} })).toBe(true);
    expect(
      isRpcResponseMessage({ requestId: 2, error: { message: "failed" } }),
    ).toBe(true);
    expect(isRpcResponseMessage({ requestId: 2, result: {}, error: {} })).toBe(
      false,
    );
  });

  it("only accepts known request envelopes", () => {
    expect(
      isMainToWorkerMessage({ kind: "add_fonts", requestId: 1, payload: {} }),
    ).toBe(true);
    expect(
      isMainToWorkerMessage({ kind: "compile", requestId: 1, payload: {} }),
    ).toBe(true);
    expect(isMainToWorkerMessage({ kind: "compile", requestId: 1 })).toBe(
      false,
    );
    expect(isMainToWorkerMessage({ kind: "add_fonts", requestId: 1 })).toBe(
      false,
    );
    expect(isMainToWorkerMessage({ kind: "unknown", requestId: 1 })).toBe(
      false,
    );
    expect(
      isMainToWorkerMessage({ kind: "compile", requestId: 1, payload: 1 }),
    ).toBe(false);
  });

  it("only accepts well-formed worker fetch events", () => {
    expect(
      isWorkerEventMessage({
        kind: "web_fetch",
        payload: { request: { path: "main.typ", kind: "project" } },
      }),
    ).toBe(true);
    expect(isWorkerEventMessage({ kind: "web_fetch" })).toBe(false);
    expect(
      isWorkerEventMessage({
        kind: "web_fetch",
        payload: { request: { path: 1, kind: "project" } },
      }),
    ).toBe(false);
    expect(isWorkerEventMessage({ kind: "unknown" })).toBe(false);
  });
});
