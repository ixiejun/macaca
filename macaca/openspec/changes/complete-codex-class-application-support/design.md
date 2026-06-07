# Design: Complete Codex-class Application Support

## Context

Codex exposes a local agent platform with Thread/Turn/Item primitives,
bidirectional app-server protocol, filesystem/process/sandbox operations,
approvals, hooks, skills, MCP, plugins, model catalog, config, memory, realtime,
remote environments, diagnostics, and TUI/IDE clients. Macaca should support
this class of application through generic OS services, not by embedding Codex
product behavior.

The Macaca constitution requires:

- Kernel owns only invariants.
- Replaceable capabilities live behind services, plugins, optional modules, or
  application framework contracts.
- SDK and shells call facades; they do not construct providers.
- Web, CLI, frontend, and app protocol gateways are presentation/transport
  adapters.
- Every service call requires trace, policy before side effects, structured
  unavailable/unsupported/denied/failure states, sanitized logs, and audit.

## Goals

- Provide the complete generic capability substrate for a Codex-class coding
  application.
- Keep all product-specific behavior in application packages.
- Make all new capabilities provider-neutral, service-owned, traceable,
  auditable, and replaceable.
- Support built-in, plugin, remote, mock, and unavailable providers.
- Prove the implementation with a real application-neutral coding workflow.

## Non-Goals

- Do not copy Codex product prompts, UI copy, keyboard behavior, model defaults,
  provider defaults, or business workflow into Macaca OS.
- Do not make Web, CLI, frontend, or app protocol gateway semantic owners.
- Do not require optional realtime, remote environment, Web3, EVM, paid package,
  or specific plugin providers for base OS startup.
- Do not bypass `service.tool`, `service.llm`, policy, resource, entitlement,
  or audit chains.

## Architecture

```text
Application package
  -> Application manifest capability declarations
  -> SDK/SystemFacade/focused clients
  -> System services
  -> Runtime-host provider factories and adapters
  -> Service runtime decorators
  -> Kernel service-call, identity, policy, resource, trace, and audit invariants
```

New or upgraded service families:

- `service.interaction`
- `service.app_protocol`
- `service.file`
- `service.process`
- `service.sandbox`
- `service.approval`
- `service.hook`
- `service.config`
- `service.plugin_marketplace`
- `service.code_intelligence`
- `service.git`
- `service.review`
- `service.diagnostics`
- `service.realtime`
- `service.remote_environment`
- upgraded `service.llm`
- upgraded `service.mcp`
- upgraded `service.skill`
- integration with `service.tool`

## Design Patterns

- **Facade:** every service receives a focused SDK client and SystemFacade
  bridge. Shells use these clients only.
- **Command:** every operation uses typed command/result DTOs in provider-neutral
  crates and carries trace context.
- **Adapter/Bridge:** provider implementations adapt local, remote, plugin,
  MCP, filesystem, process, PTY, Git, analyzer, protocol, and realtime
  transports into service contracts.
- **Strategy:** routing, sandbox profiles, permission profiles, model/provider
  selection, approval reviewers, hook selection, analyzer selection, retry, and
  degradation are swappable strategies.
- **Decorator:** trace, policy, resource, entitlement, budget, approval, timeout,
  redaction, output bounds, and metering wrap privileged calls before side
  effects.
- **State:** service, thread, turn, item, process, sandbox environment,
  approval, hook, plugin, MCP, skill, review, realtime, and remote environment
  lifecycles are explicit state machines.
- **Observer:** EventLog, SSE, app protocol notifications, filesystem watch,
  process output, approval updates, hook results, provider health, and
  diagnostics are subscribable.
- **Memento:** interaction history, snapshots, patches, rollback markers,
  artifacts, approvals, config versions, and diagnostic bundles are replayable.
- **Specification:** path, network, permission, package, hook, entitlement,
  optional module, and dependency-boundary rules are executable specifications.
- **Abstract Factory:** runtime-host composition roots construct providers.
- **Null Object:** unavailable providers return structured unavailable states.

## Service Requirements

Every new service must:

- Own a stable descriptor, lifecycle, health, snapshot, command surface, and
  focused SDK client.
- Require trace context on all calls.
- Run policy/resource/budget/entitlement checks before privileged side effects.
- Emit sanitized logs at command accepted, policy evaluated, provider dispatch
  started/completed/failed, resource lease acquired/released, artifact stored,
  audit appended, and event emitted.
- Return structured unavailable, unsupported, denied, and failure states.
- Bound all snapshots, logs, and event payloads.
- Store large or sensitive outputs as artifact refs.
- Support mock and unavailable providers for tests and optional module absence.

## Shell Requirements

Web, CLI, frontend, and app protocol surfaces may:

- Parse input.
- Call SystemFacade or focused clients.
- Render Thread/Turn/Item streams, process output, file changes, approvals,
  diagnostics, tool traces, plugin/MCP/skill status, and app UI.
- Subscribe to typed events.

They must not:

- Decide approval or policy.
- Own filesystem/process/sandbox semantics.
- Own plugin, MCP, skill, Git, review, diagnostics, model, or tool execution.
- Hardcode Codex-like application workflows.

## Migration Plan

1. Add DTOs, service descriptors, focused clients, and unavailable clients.
2. Implement local provider-backed services incrementally behind the new
   contracts.
3. Wire service-backed descriptors into `service.tool`.
4. Add shell adapters only after service contracts exist.
5. Add integration proof and boundary gates.
6. Preserve existing `/api/chat/v2`, YAML, WASM, GenUI, scheduler, memory,
   skill, MCP, and industrial tools behavior throughout migration.

## Risks and Mitigations

- Risk: scope collapses into small skeleton-only services.
  Mitigation: tasks require real provider-backed workflow proof before
  completion.
- Risk: shell protocol becomes semantic owner.
  Mitigation: app protocol gateway is restricted to transport adaptation and
  focused client calls.
- Risk: coding workflow leaks into OS.
  Mitigation: proof application declares capabilities; OS services remain
  application-neutral.
- Risk: optional provider absence causes fake success.
  Mitigation: every optional provider needs Null Object unavailable behavior
  and tests.
- Risk: raw prompts, file contents, secrets, provider payloads, or outputs leak.
  Mitigation: redaction decorators, bounded payloads, artifact refs, and audit
  replay tests.

## Completion Boundary

This change is complete only when a Macaca application can run a real
Codex-class coding workflow through generic services: start a thread, stream
turn/items, read and patch files, run sandboxed commands, invoke tools/MCP/skill
providers, request approvals, run hooks, inspect code, apply patches, review
results, emit diagnostics, and replay audit evidence. Catalog-only or
descriptor-only implementations do not satisfy this change.
