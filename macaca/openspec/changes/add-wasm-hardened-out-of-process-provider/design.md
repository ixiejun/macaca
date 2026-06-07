## Context

The existing hardened envelope mock proves the data shape. A real provider must
use the same semantics while moving execution out of process.

## Goals / Non-Goals

- Goals: daemon transport abstraction, health checks, timeout/cancellation,
  backpressure, malformed response rejection, crash recovery, and sanitized
  diagnostics.
- Non-Goals: engine-specific public ABI, OS-specific sandbox policy in public
  crates, application-specialized daemon behavior, Web/CLI daemon ownership, or
  kernel-owned process lifecycle.

## Decisions

- Use Bridge and Adapter for daemon transport.
- Use Strategy through the existing provider registry.
- Use Null Object fail-closed behavior when the daemon is unavailable.
- Keep daemon request and response envelopes sanitized and provider-neutral.
- Treat the daemon as a deployment profile, not as a new Application ABI.

## Governance

The hardened provider belongs to `macaca-runtime-host`, matching the Route C
runtime-host service ownership rule. The kernel only sees provider-neutral
service/runtime invariants, and presentation shells only receive sanitized
status through existing service/facade paths.

## Risks / Trade-offs

- Out-of-process execution adds operational complexity. Mitigation: implement
  deterministic local transport tests before OS-specific hardening.
- Daemon failures can obscure root cause. Mitigation: stable reason codes,
  trace-required dispatch, and sanitized daemon health events.

## Migration Plan

Deployment profiles can select the hardened provider after conformance tests
pass. Existing in-process providers remain available for development.
