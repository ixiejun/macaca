## Context

The runtime provider contract and admission control plane are complete. This change provides the first executable in-process provider while preserving the previously established boundary: concrete engine APIs remain private to the runtime-host provider implementation.

## Goals

- Compile, instantiate, and invoke minimal WASM module exports through the provider-neutral session trait.
- Keep concrete engine types and errors out of `macaca-proto`, SDK, Application Framework, Web, and CLI surfaces.
- Provide deterministic compiled artifact cache keys that do not store raw bytes.
- Emit sanitized diagnostics and logs for compile, cache, instantiate, invoke, trap, and shutdown nodes.
- Preserve the unavailable provider as the fail-closed fallback.

## Non-Goals

- Do not implement a hardened out-of-process provider.
- Do not implement full WASI policy.
- Do not expose raw guest stdout/stderr, memory dumps, raw payloads, raw manifests, secrets, environment values, API keys, prompts, or private keys.
- Do not make Kernel depend on any WASM engine.

## Decisions

- Decision: Keep the engine dependency private to `macaca-runtime-host`.
  Rationale: runtime-host owns execution providers, while public contracts stay provider-neutral and replaceable.

- Decision: Use Strategy for artifact loading, compile cache, engine adapter, and error mapping.
  Rationale: later providers can swap file/package loaders, cache stores, engines, and diagnostic policies independently.

- Decision: Use an Adapter around the in-process engine.
  Rationale: engine-specific APIs and traps are mapped into `WasmRuntimeErrorKind` and `ApplicationHostCommandResult`.

- Decision: Use Null Object fallback.
  Rationale: default provider construction failures return unavailable provider behavior rather than silently succeeding.

- Decision: Use cache Mementos keyed by digest, ABI version, capability fingerprint, and profile fingerprint.
  Rationale: cache entries must be auditable and deterministic without retaining raw WASM bytes.

## Trace / Audit

Provider construction, availability, artifact load, cache hit/miss, compile, instantiate, invoke, trap, and shutdown must log trace id, application id, ability id, runtime kind, artifact hash prefix, and reason code. Logs and diagnostics must never include raw WASM bytes, raw guest payloads, raw stdout/stderr, raw memory dumps, raw manifests, secrets, environment values, API keys, prompts, private keys, or unbounded provider output.
