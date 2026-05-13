## Context

Current code logs many key nodes, but there is no single production sink
contract for runtime events and metrics.

## Goals / Non-Goals

- Goals: Observer sink trait, sanitized event DTOs, test sink, tracing sink,
  emission at all WASM runtime decision points, and redaction tests.
- Non-Goals: one vendor-specific dashboard, raw payload export,
  app-specific telemetry schema, kernel-owned provider telemetry, or Web/CLI
  telemetry semantics.

## Decisions

- Use Observer for sink fan-out.
- Use Memento-style event DTOs so telemetry is serializable and auditable.
- Keep sink failures non-fatal unless policy explicitly marks audit persistence
  as mandatory.
- Emit safe subject, trace id, event kind, status, reason code, duration, and
  sanitized diagnostics only.

## Governance

Observability belongs to runtime-host trace/audit emission and provider-neutral
event DTOs. Presentation shells may display telemetry, but they must not define
WASM runtime event semantics or construct provider telemetry backends.

## Risks / Trade-offs

- Telemetry can leak sensitive data. Mitigation: sanitizer tests and safe-subject
  fields only.
- Sink coupling can make runtime hard to replace. Mitigation: depend on a small
  trait and provide in-memory/tracing adapters.

## Migration Plan

Existing logs remain. Runtime provider paths progressively emit structured
events to the configured sink.
