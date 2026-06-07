## Context

The provider-neutral runtime contract already exists. The new provider must add
production Component Model execution without exposing concrete engine types
outside `macaca-runtime-host`.

## Goals / Non-Goals

- Goals: Component Model validation, WIT package matching, canonical ABI
  dispatch, host import bridge integration, engine-enforced limits, sanitized
  traps, and traceable execution events.
- Non-Goals: public Wasmtime or WasmEdge DTOs, kernel-owned WASM execution,
  application-specific imports, presentation shell provider construction, or
  hardcoded application/workflow/provider names.

## Decisions

- Use Strategy and Abstract Factory through `WasmApplicationRuntimeProvider`.
- Use Adapter for the concrete engine boundary.
- Use Bridge for host imports so guest calls continue through service runtime
  and capability/policy checks.
- Keep the existing in-process core-WASM provider for compatibility tests.
- Add an engine dependency only to `macaca-runtime-host` after this proposal is
  approved; never add it to `macaca-proto`, `macaca-app`, `macaca-sdk`,
  `macaca-kernel`, Web, or CLI.
- Treat every compile, instantiate, invoke, trap, timeout, and resource decision
  as an auditable event with sanitized logs.

## Governance

This change belongs to runtime-host service ownership, not the microkernel.
It must not introduce a Route C allowlist exception. If an implementation
attempt requires one, the implementation must stop and update OpenSpec plus
the allowlist/test allowlist before proceeding.

## Risks / Trade-offs

- Engine dependency increases build surface. Mitigation: keep dependency
  private, runtime-host-owned, and optionally feature-gated.
- Component Model binding errors can leak payload details. Mitigation: sanitize
  all trap and ABI diagnostics before logs, traces, telemetry, or results.
- Engine-level resource limits can differ by backend. Mitigation: define
  provider-neutral reason codes and map backend behavior through Adapter.

## Migration Plan

Existing users keep the default provider. Deployment profiles opt into the
Component Model provider when runtime capabilities and admission checks pass.
