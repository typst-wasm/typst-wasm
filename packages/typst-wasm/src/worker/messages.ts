import type {
  RpcRequestMessage,
  RpcResponseMessage,
  TypstWorkerProtocol,
} from "./protocol";
import type { EngineFetchRequest } from "../engine/types";

export type MainToWorkerMessage = RpcRequestMessage<TypstWorkerProtocol>;

export type WorkerEventMessage = {
  kind: "web_fetch";
  payload: { request: EngineFetchRequest };
};

export type WorkerToMainMessage = RpcResponseMessage | WorkerEventMessage;

const commandKinds = [
  "init",
  "add_fonts",
  "compile",
] as const satisfies readonly (keyof TypstWorkerProtocol)[];
const payloadCommands = new Set<keyof TypstWorkerProtocol>([
  "init",
  "add_fonts",
  "compile",
]);

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null;

const isCommandKind = (value: unknown): value is keyof TypstWorkerProtocol =>
  typeof value === "string" &&
  (commandKinds as readonly string[]).includes(value);

export const isMainToWorkerMessage = (
  value: unknown,
): value is MainToWorkerMessage => {
  if (!isRecord(value)) return false;
  if (!isCommandKind(value.kind) || typeof value.requestId !== "number")
    return false;
  const hasPayload = "payload" in value;
  if (payloadCommands.has(value.kind))
    return hasPayload && isRecord(value.payload);
  return !hasPayload;
};

export const isRpcResponseMessage = (
  value: unknown,
): value is RpcResponseMessage => {
  if (!isRecord(value) || typeof value.requestId !== "number") return false;
  return "result" in value !== "error" in value;
};

export const isWorkerEventMessage = (
  value: unknown,
): value is WorkerEventMessage => {
  if (!isRecord(value) || value.kind !== "web_fetch") return false;
  if (!isRecord(value.payload) || !isRecord(value.payload.request))
    return false;
  const request = value.payload.request;
  return (
    typeof request.path === "string" &&
    (request.kind === "project" ||
      request.kind === "package" ||
      request.kind === "url")
  );
};
