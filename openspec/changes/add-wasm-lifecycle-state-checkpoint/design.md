## Context
Macaca already has provider-neutral WASM runtime contracts, a default in-process provider, sandbox resource governance, and a ServiceRuntime-backed host import portal. This change adds the lifecycle layer needed by long-running WASM applications without adding persistent storage, live migration, or provider-specific process control.

## Goals / Non-Goals
- Goals: represent lifecycle transitions as typed commands, keep transition validation centralized, expose sanitized checkpoint/restore/upgrade/rollback mementos, and emit traceable audit/log metadata.
- Non-Goals: persist checkpoint artifacts, serialize raw WASM memory, migrate live processes across machines, or hard-code application, workflow, driver, or business names.

## Decisions
- Decision: Use State plus Specification for `WasmLifecycleState` and transition validation.
  Rationale: legal transitions stay explicit and auditable instead of being scattered across provider dispatch branches.
- Decision: Use Command DTOs for lifecycle actions and Memento DTOs for checkpoint/restore/upgrade/rollback outputs.
  Rationale: provider callers can request lifecycle operations through stable data without depending on a concrete engine.
- Decision: Use Observer-style audit records and tracing logs at every key lifecycle edge.
  Rationale: runtime behavior remains traceable while sanitized metadata avoids raw payload, memory, prompt, secret, and environment leakage.
- Decision: Default in-process provider returns structured unsupported for pause/resume/drain when the engine lacks real suspension support.
  Rationale: fail-closed unsupported results are safer than pretending lifecycle state changed when guest execution was not actually suspended or resumed.

## Risks / Trade-offs
- State graph complexity -> keep the first implementation explicit and small, with unsupported results for engine features that are not real yet.
- Checkpoint leakage -> portable checkpoint DTOs store metadata only and sanitize output keys/values.
- ABI-breaking upgrades -> upgrade/rollback reports require artifact/hash/ABI metadata and reject ABI mismatches.

## Migration Plan
Existing invocation and host import behavior remains compatible. New lifecycle methods are exposed as trait defaults and provider modules, so callers can adopt them incrementally while unavailable/default providers continue to fail closed.
