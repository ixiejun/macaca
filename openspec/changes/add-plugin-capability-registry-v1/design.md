# Design: Plugin Capability Registry v1

## Context

Plugin Runtime v0 introduced plugin-owned service descriptors but did not make plugin capabilities first-class. This change adds the capability plane required for real plugin extensibility while still avoiding arbitrary third-party code execution.

## Goals

- Make plugin-provided capabilities discoverable before runtime activation.
- Support all core Macaca extension categories through provider-neutral descriptors.
- Provide conflict policy and ownership cleanup.
- Canonicalize built-in service adapters as plugin capabilities.
- Keep capability behavior in services, not the registry.
- Prepare future SDK/host execution phases.

## Non-Goals

- No arbitrary external plugin execution.
- No hook execution; Hook Bus is a separate proposal.
- No Store marketplace.
- No direct Web/CLI provider construction.

## Architecture

```text
Plugin Manifest / Repository Snapshot
  -> Capability Contract Discovery
  -> Conflict Policy
  -> Plugin Capability Registry
  -> ServiceRuntime / Capability Call Envelope
  -> Built-in Service Adapter or Structured Unavailable
```

## Design Patterns

- **Registry**: stores capability ownership and descriptor indexes.
- **Composite**: one plugin can own a set of capabilities and services.
- **Specification**: validates capability schemas, visibility, permissions, resources, and conflicts.
- **Strategy**: conflict policy and slot policy are replaceable.
- **Adapter**: existing built-in services become canonical plugin capability descriptors.
- **Command**: capability registration, activation, deactivation, and call attempts are typed commands.
- **Null Object**: missing optional capability returns structured unavailable.
- **Observer**: capability registration/call/failure emits trace/audit.

## Capability Kinds

The registry must support:

- tool
- hook
- driver
- gateway
- skill
- mcp
- memory
- context
- llm_provider
- observability
- http_route
- cli_command
- custom

## Boundary Decisions

- `macaca-proto` owns capability descriptor DTOs.
- `macaca-kernel` may store plugin ownership maps but does not route calls or implement behavior.
- `macaca-runtime-host` owns capability admission, conflict policy, and runtime registration orchestration.
- Service crates own their domain adapter descriptors.
- SDK exposes query/call clients without concrete service dependencies.

## Conflict Policy

The conflict system must fail closed for:

- duplicate active tool names without namespace.
- duplicate gateway routes.
- duplicate HTTP route path/method pairs.
- duplicate CLI command names.
- multiple owners for exclusive slots such as selected default memory/context/provider without priority policy.

Conflicts must return structured diagnostics naming capability ids and plugin ids without leaking config or secrets.

## Trace And Audit

Capability registration, deactivation, cleanup, conflict rejection, and call routing attempts must emit structured logs and trace/audit records with plugin id, capability id, capability kind, operation, decision, trace id, and structured error code.

## Risks

- **Risk: Registry becomes a behavior hub.** Mitigation: registry stores descriptors and ownership only.
- **Risk: Built-in adapters change existing behavior.** Mitigation: adapters are additive and current runtime paths remain until later migrations.
- **Risk: Capability conflicts block base OS.** Mitigation: optional capabilities degrade; required conflicts fail with diagnostics.
