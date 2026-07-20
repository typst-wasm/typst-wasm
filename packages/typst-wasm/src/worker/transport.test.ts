import { describe, expect, it, vi } from "vitest";
import type { WorkerHost } from "./host";
import { makeWorkerTransport } from "./transport";

class FakeWorker implements WorkerHost {
  onMessage: ((data: unknown) => void) | null = null;
  onError: ((cause: unknown) => void) | null = null;
  messages: unknown[] = [];

  listen(
    onMessage: (data: unknown) => void,
    onError: (cause: unknown) => void,
  ) {
    this.onMessage = onMessage;
    this.onError = onError;
  }

  postMessage(message: unknown) {
    this.messages.push(message);
  }

  terminate() {}

  emit(data: unknown) {
    this.onMessage?.(data);
  }
}

describe("worker transport", () => {
  it("forwards outbound messages to worker", () => {
    const worker = new FakeWorker();
    const transport = makeWorkerTransport(
      worker,
      () => undefined,
      () => undefined,
      () => undefined,
    );

    transport.post({
      kind: "compile",
      requestId: 1,
      payload: { options: { format: "html" } },
    });

    expect(worker.messages).toEqual([
      {
        kind: "compile",
        requestId: 1,
        payload: { options: { format: "html" } },
      },
    ]);
  });

  it("reports invalid inbound messages", () => {
    const worker = new FakeWorker();
    const error = vi.fn();
    makeWorkerTransport(
      worker,
      () => undefined,
      () => undefined,
      () => undefined,
      {
        error,
        debug: vi.fn(),
      },
    );

    worker.emit({ kind: "unknown_event" });

    expect(error).toHaveBeenCalledWith(
      "Received invalid message from Typst worker",
      {
        kind: "unknown_event",
      },
    );
  });

  it("forwards only valid inbound worker messages", () => {
    const worker = new FakeWorker();
    const messages: unknown[] = [];
    makeWorkerTransport(
      worker,
      () => undefined,
      (message) => messages.push(message),
      () => undefined,
    );

    worker.emit({ kind: "unknown_event" });
    worker.emit({
      kind: "web_fetch",
      payload: { request: { path: "main.typ", kind: "project" } },
    });

    expect(messages).toEqual([
      {
        kind: "web_fetch",
        payload: { request: { path: "main.typ", kind: "project" } },
      },
    ]);
  });
});
