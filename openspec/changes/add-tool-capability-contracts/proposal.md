# Change: Add industrial tool capability contracts

## Why

Macaca has service-backed Driver, Skill, MCP, Memory, Task, Scheduler, Gateway, and runtime tool primitives, but it does not yet have one provider-neutral industrial Tools contract for descriptor planning, visibility diagnostics, invocation routing metadata, result classes, artifact refs, provider health, and audit refs.

Without this contract, each layer keeps re-creating partial tool metadata and the platform cannot safely scale from "some callable tools" to a complete industrial Tools system.

## What Changes

- Add provider-neutral Tool Capability DTOs for industrial descriptors, tool plans, hidden diagnostics, tool families, toolsets, availability expressions, policy refs, result classes, artifact refs, provider status, and audit refs.
- Add `service.tool` command/result contracts for planning, snapshots, toolset resolution, invocation, cancellation, invocation status, result retrieval, artifact access, provider health, policy explanation, and audit query.
- Add SDK `SystemToolClient` with service-backed and unavailable Null Object behavior.
- Preserve concrete ownership of Driver, Skill, MCP, Memory, Task, Scheduler, Gateway, Store, and other provider services.
- Establish explicit design-pattern vocabulary for later implementation: Facade, Command, Adapter/Bridge, Strategy, Decorator, State, Observer, Memento, Specification, Abstract Factory, and Null Object.

## Impact

- Affected specs: `tool-capability-contracts`, `sdk-system-facade`, `service-runtime`
- Affected code: `macaca-proto`, `macaca-sdk`, `macaca-runtime-host`
- Follow-up changes: all later industrial Tools proposals depend on these DTOs and command names.

## Constraints

- Must follow `macaca-os-architecture-governance.md`, `macaca-os-microkernel-boundaries.md`, and `macaca-os-serviceization-allowlist.md`.
- Must not move provider lifecycle or business semantics into the kernel, SDK, Web, CLI, frontend, or applications.
- Must not hardcode application names, workflows, provider names, model names, driver names, gateway names, chain names, or business-domain branches.
- Must not expose raw secrets, prompts, manifests, credentials, headers, env values, private keys, raw provider payloads, or unbounded output in descriptors, diagnostics, snapshots, logs, or audit records.
