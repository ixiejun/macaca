## Context

The WASM ABI skeleton validates metadata-only packages, and the runtime provider contract defines execution-plane descriptors and unavailable behavior. This change adds the control-plane admission model between package metadata and future runtime sessions.

## Goals

- Admit WASM packages through composable Specification checks.
- Negotiate ABI versions without executing or loading raw WASM bytes.
- Produce sanitized Memento-style admission and compatibility reports for Web, CLI, SDK, Store, and certification tooling.
- Preserve the legacy metadata-only skeleton as an Adapter that produces the new report.
- Keep every failure traceable through stable reason codes.

## Non-Goals

- Do not compile, instantiate, or execute WASM.
- Do not download artifacts or verify full Store signature chains.
- Do not dispatch host imports to services.
- Do not generate guest SDK bindings.

## Decisions

- Decision: Extend `macaca-proto` with DTOs, not runtime logic.
  Rationale: artifact and ABI negotiation facts must be visible to SDK, Store, Web/CLI, and runtime-host without depending on `macaca-app`.

- Decision: Implement admission in `macaca-app::certification` as Specification + Visitor.
  Rationale: certification already owns provider-neutral manifest/ABI inspection and produces sanitized reports.

- Decision: Use report DTOs as Mementos.
  Rationale: admission evidence must be replayable and auditable without raw artifacts, raw manifests, or runtime handles.

- Decision: Keep the current WASM descriptor as a legacy Adapter.
  Rationale: metadata-only admission remains useful before real engines exist, but the output must converge on the new admission report.

## Trace / Audit

Every admission run records package id, runtime kind, ABI version, trace id, status, diagnostic count, and reason codes. Logs and reports must not include raw WASM bytes, raw manifest bodies, raw host payloads, secrets, env values, API keys, prompts, private keys, raw signatures, or unbounded provider output.
