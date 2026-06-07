## Context

The architecture report identifies WASM as the long-term application binary boundary. This proposal formalizes the ABI skeleton without executing third-party code.

## Goals

- Define WIT/schema aligned with `ApplicationImport` and `ApplicationExport`.
- Add WASM component descriptors and SDK scaffold helpers.
- Add unavailable-safe host factory and host implementation.
- Prove metadata admission and structured unavailable execution behavior.

## Non-Goals

- Do not add a real WASM runtime dependency.
- Do not execute third-party WASM.
- Do not implement language bindings beyond scaffold placeholders unless they are data-only.
- Do not bypass service runtime, permission, trace, or policy boundaries.

## Decisions

- Decision: Use Bridge between WASM guest imports and Macaca service commands.
  Rationale: guest code must not depend on provider/runtime internals.

- Decision: Use Command for every host import.
  Rationale: host calls need trace, metadata, policy, and structured results.

- Decision: Use Abstract Factory for application host creation by runtime kind.
  Rationale: future real WASM host can replace unavailable host without changing callers.

- Decision: Use Null Object for current unavailable WASM execution.
  Rationale: missing optional runtime must be explicit, traceable, and non-fatal.

- Decision: Keep schema and Rust DTOs aligned through tests.
  Rationale: ABI drift is dangerous for multi-language SDKs.

## Risks / Trade-offs

- Risk: WIT/schema gets ahead of Rust DTOs.
  Mitigation: add tests that assert import/export names align.

- Risk: unavailable host is mistaken for real execution.
  Mitigation: status, diagnostics, trace logs, and fixture names must explicitly say runtime unavailable.

- Risk: real WASM runtime dependency sneaks in.
  Mitigation: forbid heavy runtime dependency until a later OpenSpec approves it.

## Migration Plan

1. Add WIT/schema file.
2. Add DTO alignment tests.
3. Add WASM descriptor and SDK scaffold helpers.
4. Add runtime-host unavailable host factory.
5. Add integration fixture and tests.

## Trace / Audit

Every WASM host operation must log operation, trace id, package/application id, runtime kind, status, and reason code. Logs must not include raw WASM bytes, raw payload, secrets, env, API keys, prompts, private keys, or unbounded provider output.
