## Context

This proposal adds the environment and gateway execution substrate required by industrial tools. It builds on `service.tool` contracts and invocation routing.

Existing tools often assume a local process, a workspace root, or an external service. Industrial execution needs a generic model where tools request an environment capability and the platform chooses an admitted provider through policy and descriptors.

## Goals

- Model runtime environments as provider-backed capabilities.
- Support health, cleanup, artifact roots, process handles, secret injection policy, network policy, filesystem policy, and resource policy.
- Add optional managed gateway provider registration and health.
- Support local workspace, sandbox, Docker, SSH/remote, WASM host import, browser sandbox, per-call, and session-scoped environment categories.
- Ensure managed gateway providers can add industrial tools without OS control-flow branches.

## Non-Goals

- Do not make any specific gateway mandatory.
- Do not hardcode provider names in OS routing.
- Do not add all industrial tool families in this proposal.
- Do not move tool semantics into shell code.

## Decisions

### Abstract Factory

Runtime-host composition roots bootstrap environment and gateway providers. Built-in, plugin, remote, mock, managed, and unavailable providers share the same contract.

### Strategy

Provider selection and gateway routing are strategies driven by descriptor/config/policy data. OS code must not branch on concrete provider product names.

### State

Environment lifecycle is explicit: unavailable, starting, ready, busy, degraded, cleaning, failed, and stopped.

### Decorator

Resource, filesystem, network, secret injection, metering, trace, and audit behavior wrap environment use before side effects.

### Null Object

Missing providers return structured unavailable diagnostics and do not fake success.

## Trace, Audit, And Logging Requirements

Environment and gateway code must log provider id, environment id, lifecycle state, resource scope, health state, cleanup status, artifact root refs, metering refs, and stable reason codes. It must not log raw secrets, env values, headers, provider payloads, or unbounded output.
