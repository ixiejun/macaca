## Context

The previous WASM skeleton approved metadata admission and unavailable-safe host behavior, but it intentionally did not define a reusable runtime provider boundary. This change keeps the next step additive: it defines the contract that future Wasmtime, WasmEdge, Wasmer, process-isolated, or remote providers can implement later without changing callers.

## Goals

- Keep the public execution contract provider-neutral and engine-neutral.
- Represent runtime capabilities, availability, profiles, sessions, and diagnostics as serializable DTOs.
- Preserve fail-closed behavior when no runtime provider is installed.
- Require trace context before any session can be created or any host command can be dispatched.
- Keep diagnostics sanitized and audit-friendly.

## Non-Goals

- Do not add a real WASM execution engine.
- Do not instantiate, compile, or execute guest WASM.
- Do not implement WASI resource authorization.
- Do not add out-of-process IPC for WASM runtime providers.

## Decisions

- Decision: Use Bridge between guest host imports and runtime provider sessions.
  Rationale: callers dispatch provider-neutral `ApplicationHostCommand` values while providers decide how to map them to a future engine.

- Decision: Use Abstract Factory and Strategy at the runtime-host boundary.
  Rationale: a provider can create sessions from descriptors and profiles, while future selection strategies can swap implementations without branching on concrete engine names.

- Decision: Use Null Object for missing providers.
  Rationale: absent optional runtime support must return structured unavailable results and logs instead of panicking, hanging, or pretending execution succeeded.

- Decision: Use Specification validation for session requests.
  Rationale: missing trace, application id, ability id, artifact reference, or profile must be rejected before any provider can run.

- Decision: Keep diagnostics redacted by construction.
  Rationale: diagnostics may flow into logs, traces, SDKs, and UI surfaces, so they must not carry raw bytes, raw payloads, manifests, secrets, env values, API keys, or unbounded provider output.

## Boundary

The contract belongs to the execution plane. `macaca-proto` owns data-only DTOs, and `macaca-runtime-host` owns runtime provider traits and unavailable provider behavior. Application Framework may consume descriptors and session results but must not construct concrete providers. SDK code may serialize and inspect provider-neutral DTOs but must not depend on runtime-host provider implementations.

## Trace / Audit

Provider selection, availability checks, session creation, session rejection, and command rejection must log trace id when present, runtime kind, reason code, and sanitized status. Logs must never include raw command payloads, raw WASM bytes, secrets, env values, API keys, raw manifests, or provider-specific internals.
