// Tool outputs may contain command output, service diagnostics, or provider
// errors.  The Workbench must redact secret-looking keys before it is shown in
// the UI or sent back to the LLM as a tool-result continuation.  The default
// path intentionally preserves the full user-visible payload because the
// application execution protocol already separates public display evidence from
// hidden provider internals; callers that need a bounded operational error
// summary can still pass explicit byte/line budgets.
const DEFAULT_MAX_BYTES = 4096;
const DEFAULT_MAX_LINES = 80;
const SECRET_PATTERN = /(api[_-]?key|token|password|credential|private[_-]?key|authorization)/gi;

export function sanitizeToolOutput(value, options = {}) {
  const preserveFull = options.preserveFull ?? true;
  const maxBytes = options.maxBytes ?? (preserveFull ? null : DEFAULT_MAX_BYTES);
  const maxLines = options.maxLines ?? (preserveFull ? null : DEFAULT_MAX_LINES);
  const raw = typeof value === "string" ? value : JSON.stringify(value ?? null, null, 2);
  const redacted = raw.replace(SECRET_PATTERN, "[redacted]");
  const lines = redacted.split(/\r?\n/);
  const lineBounded = maxLines && lines.length > maxLines ? lines.slice(0, maxLines).join("\n") : redacted;
  const encoded = new TextEncoder().encode(lineBounded);
  const truncated = Boolean((maxBytes && encoded.length > maxBytes) || (maxLines && lines.length > maxLines));
  const text =
    maxBytes && encoded.length > maxBytes
      ? new TextDecoder().decode(encoded.slice(0, maxBytes))
      : lineBounded;

  return {
    text,
    byte_len: new TextEncoder().encode(redacted).length,
    truncated,
  };
}

export function sanitizeError(error) {
  const message = error instanceof Error ? error.message : String(error);
  return sanitizeToolOutput(message, { maxBytes: 1024, maxLines: 12 });
}
