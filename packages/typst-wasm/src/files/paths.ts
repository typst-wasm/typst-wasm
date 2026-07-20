/** Normalize a project path using the same rules as the Rust engine. */
export const normalizeProjectPath = (input: string): string => {
  if (input.trim().length === 0) throw new Error("path cannot be empty");
  if (input.startsWith("/")) throw new Error("absolute paths are not allowed");

  const parts: string[] = [];
  for (const part of input.split("/")) {
    if (part === "" || part === ".") continue;
    if (part === "..") {
      if (parts.length === 0) throw new Error("path escapes the project root");
      parts.pop();
    } else {
      parts.push(part);
    }
  }
  if (parts.length === 0) throw new Error("path cannot be empty");
  return parts.join("/");
};
