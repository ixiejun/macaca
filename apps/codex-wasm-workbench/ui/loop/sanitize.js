// Tool outputs may contain command output, service diagnostics, or provider
// errors.  The Workbench must bound and redact that material before it is shown
// in the UI or sent back to the LLM as a tool-result continuation.
const DEFAULT_MAX_BYTES = 4096;
const DEFAULT_MAX_LINES = 80;
const SECRET_PATTERN = /(api[_-]?key|token|password|credential|private[_-]?key|authorization)/gi;

export function sanitizeToolOutput(value, options = {}) {
  const maxBytes = options.maxBytes ?? DEFAULT_MAX_BYTES;
  const maxLines = options.maxLines ?? DEFAULT_MAX_LINES;
  const raw = typeof value === "string" ? value : JSON.stringify(value ?? null, null, 2);
  const redacted = raw.replace(SECRET_PATTERN, "[redacted]");
  const lines = redacted.split(/\r?\n/);
  const lineBounded = lines.length > maxLines ? lines.slice(0, maxLines).join("\n") : redacted;
  const encoded = new TextEncoder().encode(lineBounded);
  const truncated = encoded.length > maxBytes || lines.length > maxLines;
  const text =
    encoded.length > maxBytes
      ? new TextDecoder().decode(encoded.slice(0, maxBytes))
      : lineBounded;

  return {
    text,
    byte_len: encoded.length,
    truncated,
  };
}

export function sanitizeError(error) {
  const message = error instanceof Error ? error.message : String(error);
  return sanitizeToolOutput(message, { maxBytes: 1024, maxLines: 12 });
}
