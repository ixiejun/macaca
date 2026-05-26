## Context

Previous proposals create contracts, planning, invocation, environments, gateway, observability, and shell diagnostics. This proposal fills out the actual application-neutral industrial tool surface required by `docs/macaca-industrial-tools-system-design.md`.

The objective is not to add every possible provider as a built-in. The objective is to ensure every required family has a service-owned or provider-backed path: existing service, MCP, plugin, managed gateway, runtime adapter, or structured unavailable provider.

## Goals

- Provide rich generic tool families for real complex work.
- Use existing services and extension points where possible.
- Prove a realistic multi-family industrial workflow.
- Keep provider lifecycle and invocation service-owned.
- Keep optional providers optional.

## Non-Goals

- Do not hardcode application-specific business logic.
- Do not force one provider for every family.
- Do not bypass service runtime, policy, trace, result handling, telemetry, or audit.
- Do not add provider product names as OS control-flow branches.

## Decisions

### Adapter / Bridge

Existing services, MCP servers, plugins, gateway providers, and runtime adapters enter as family providers through descriptor contributors and invocation routes.

### Strategy

Provider selection is policy and descriptor driven. The same family can be served by multiple providers.

### Abstract Factory

Runtime-host composition roots bootstrap optional provider adapters and unavailable providers.

### Null Object

Missing family providers return structured unavailable diagnostics. They do not fake success.

### Observer and Memento

The live proof must produce traceable events, artifact refs, stable audit refs, and aggregate counts without raw model/provider output.

## Required Tool Families

- `file`
- `shell`
- `browser`
- `web`
- `memory`
- `knowledge`
- `task`
- `scheduler`
- `skill`
- `mcp`
- `media`
- `document`
- `communication`
- `enterprise_api`
- `code_execution`
- `computer_use`
- `payment_entitlement`

## Trace, Audit, And Logging Requirements

Every family provider must log provider registration, descriptor contribution, availability decision, invocation route, result class, artifact refs, and failure reason with sanitized fields. The live proof report must summarize stable refs and aggregate counts only.
