# Change: Complete AgentScope 2.0 framework contract parity

## Why

The AgentScope Java 2.0 audit found 25 capability gaps that belong in `macaca-framework` as provider-neutral contracts, adapters, state machines, evidence schemas, or testable parity gates. The existing AgentScope 2.0 upgrade proposals define the broad migration direction, but they do not yet make each missing framework-owned capability independently trackable and verifiable.

Without this closure, consumers can accidentally depend on partial AgentScope 1.0-era behavior, overbroad provider availability claims, or concrete service implementations hidden inside the framework. That would violate the Macaca OS architecture constitution and make future framework replacement harder.

This implementation is based on the latest `origin/main` after `refactor-unified-call-path-microkernel`. The unified `service.call` path and `2026-06-07-macaca-os-protocol-microkernel-target-design.md` are the architectural source of truth. AgentScope 2.0 parity must be reintroduced only as pure framework contracts plus approved runtime-host service bridges; it must not restore any kernel, web shell, direct-provider, or multi-path execution debt removed by that refactor.

## What Changes

- Add detailed OpenSpec requirements for the 25 framework-owned parity gaps found during the AgentScope Java 2.0 audit.
- Keep `macaca-framework` responsible for contracts, DTOs, event projections, middleware ABI, state machines, unavailable behavior, evidence schemas, and parity tests.
- Keep concrete LLM, memory, context, vector retrieval, filesystem, sandbox, skill, MCP, task, gateway, and provider implementations in service/runtime-host/plugin ownership.
- Require trace, audit, structured logs, capability evidence, sanitized snapshots, and deterministic unavailable/unsupported/denied behavior for every delegated capability.
- Require normal canonical naming without `2` suffixes or AgentScope 1.0 compatibility carve-outs.
- Require implementation code created under this change to include clear English comments for non-obvious behavior and key structured logs at execution boundaries.

## Impact

- Affected specs: `agent-framework-agentscope2`
- Affected code areas expected during implementation:
  - `crates/runtime/macaca-framework`
  - `crates/runtime/macaca-runtime-host` provider composition and service-backed adapters
  - provider-neutral DTO crates if shared commands/results/events need to cross service boundaries
  - framework contract tests, boundary gates, trace/audit replay tests, and provider snapshot tests
- Boundary constraints:
  - `refactor-unified-call-path-microkernel` and `2026-06-07-macaca-os-protocol-microkernel-target-design.md` SHALL remain authoritative for ownership and call paths.
  - `macaca-kernel` SHALL NOT import concrete framework/provider implementations.
  - `macaca-web` SHALL NOT regain agent execution, toolkit, runtime, provider, or service semantic ownership.
  - `macaca-framework` SHALL NOT own concrete LLM, memory, filesystem, sandbox, skill, MCP, task, gateway, payment, Web3, EVM, or application business logic.
  - Concrete provider construction SHALL remain in approved composition roots.
  - Optional provider absence SHALL be explicit and testable, never a crash, hang, silent fallback, or fake success.
