import type {
  EngineCompileOptions,
  EngineCompileSuccess,
} from "../engine/types";

const MiB = 1024 * 1024;
const INITIAL_SAB_SIZE = 1 * MiB;
const MAX_SAB_SIZE = 256 * MiB;
const DEFAULT_FETCH_TIMEOUT = 30_000;

export const SharedMemoryCommunicationStatus = {
  None: 0,
  Pending: 1,
  Error: 2,
  Success: 3,
} as const;

export const SharedMemoryCommunicationError = {
  Other: 0,
  NotFound: 1,
  Denied: 2,
  Timeout: 3,
  Unavailable: 4,
} as const;
export type SharedMemoryCommunicationError =
  (typeof SharedMemoryCommunicationError)[keyof typeof SharedMemoryCommunicationError];

export type SharedMemoryCommunicationStatus =
  (typeof SharedMemoryCommunicationStatus)[keyof typeof SharedMemoryCommunicationStatus];

export class SharedMemoryCommunication {
  dataBuf: SharedArrayBuffer;
  statusBuf: SharedArrayBuffer;
  sizeBuf: SharedArrayBuffer;
  errorBuf: SharedArrayBuffer;

  constructor() {
    this.dataBuf = new SharedArrayBuffer(INITIAL_SAB_SIZE, {
      maxByteLength: MAX_SAB_SIZE,
    });

    this.statusBuf = new SharedArrayBuffer(Int32Array.BYTES_PER_ELEMENT);

    this.sizeBuf = new SharedArrayBuffer(Uint32Array.BYTES_PER_ELEMENT);
    this.errorBuf = new SharedArrayBuffer(Int32Array.BYTES_PER_ELEMENT);
  }

  private getStatusView(): Int32Array {
    return new Int32Array(this.statusBuf);
  }

  private getSizeView(): Uint32Array {
    return new Uint32Array(this.sizeBuf);
  }

  getStatus(): SharedMemoryCommunicationStatus {
    return Atomics.load(
      this.getStatusView(),
      0,
    ) as SharedMemoryCommunicationStatus;
  }

  setStatus(status: SharedMemoryCommunicationStatus): void {
    const statusView = this.getStatusView();

    Atomics.store(statusView, 0, status);
    Atomics.notify(statusView, 0, 1);
  }

  setBuffer(buf: Uint8Array): void {
    const needed = buf.byteLength;

    if (needed > MAX_SAB_SIZE) {
      throw new RangeError(
        `File too large: ${needed} bytes. ` +
          `Maximum allowed: ${MAX_SAB_SIZE} bytes.`,
      );
    }

    if (needed > this.dataBuf.byteLength) {
      try {
        this.dataBuf.grow(needed);
      } catch (cause) {
        throw new Error(
          `Unable to grow the shared buffer from ` +
            `${this.dataBuf.byteLength} bytes to ${needed} bytes.`,
          { cause },
        );
      }
    }

    const destination = new Uint8Array(this.dataBuf, 0, needed);

    destination.set(buf);

    Atomics.store(this.getSizeView(), 0, needed);
  }

  getBuffer(): Uint8Array {
    const size = Atomics.load(this.getSizeView(), 0);

    if (size > this.dataBuf.byteLength) {
      throw new Error(
        `Invalid shared buffer size: ${size}. ` +
          `Current buffer length: ${this.dataBuf.byteLength}.`,
      );
    }

    return new Uint8Array(this.dataBuf, 0, size);
  }

  setError(error: SharedMemoryCommunicationError): void {
    Atomics.store(new Int32Array(this.errorBuf), 0, error);
  }

  getError(): SharedMemoryCommunicationError {
    return Atomics.load(
      new Int32Array(this.errorBuf),
      0,
    ) as SharedMemoryCommunicationError;
  }

  waitForStatusChange(
    expectedStatus: SharedMemoryCommunicationStatus,
    timeoutMs = DEFAULT_FETCH_TIMEOUT,
  ): boolean {
    const statusView = this.getStatusView();
    const deadline = Date.now() + timeoutMs;

    while (Atomics.load(statusView, 0) === expectedStatus) {
      const remaining = deadline - Date.now();

      if (remaining <= 0) {
        return false;
      }

      const result = Atomics.wait(statusView, 0, expectedStatus, remaining);

      if (
        result === "timed-out" &&
        Atomics.load(statusView, 0) === expectedStatus
      ) {
        return false;
      }
    }

    return true;
  }

  static hydrateObj(obj: SharedMemoryCommunication): SharedMemoryCommunication {
    const instance = Object.create(
      SharedMemoryCommunication.prototype,
    ) as SharedMemoryCommunication;

    instance.dataBuf = obj.dataBuf;
    instance.statusBuf = obj.statusBuf;
    instance.sizeBuf = obj.sizeBuf;
    instance.errorBuf = obj.errorBuf;

    return instance;
  }
}

export interface TypstWorkerProtocol {
  init: {
    request: {
      sharedMemoryCommunication: SharedMemoryCommunication;
      coreModules?: Record<string, WebAssembly.Module>;
    };
    response: void;
  };

  add_fonts: {
    request: {
      data: Uint8Array[];
    };
    response: void;
  };

  compile: {
    request: {
      options: EngineCompileOptions;
    };
    response: EngineCompileSuccess;
  };
}

export type RequestOf<M> = M extends { request: infer P } ? P : void;
export type ResponseOf<M> = M extends { response: infer R } ? R : void;

export type RpcRequestMessage<T> = {
  [K in keyof T]: {
    kind: K;
    requestId: number;
  } & (RequestOf<T[K]> extends void
    ? Record<never, never>
    : { payload: RequestOf<T[K]> });
}[keyof T];

export type RpcResponseMessage<TResult = unknown, TError = unknown> =
  | { requestId: number; result: TResult }
  | { requestId: number; error: TError };
