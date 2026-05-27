# Superpowers Plan: Complete Codex-class Application Support

Date: 2026-05-28

## Goal

Implement the full generic Macaca OS capability substrate needed to support a
Codex-class coding application as an ordinary Macaca application. This must not
be a small foundational slice and must not introduce Codex-specific OS logic.
The implementation is complete only when an application can declare and use the
full workbench capability set: interaction ledger, app protocol gateway,
filesystem, process/PTY, sandbox, approval, hooks, config/requirements,
plugin/marketplace, MCP/skill lifecycle, model catalog hardening, code
intelligence, Git/patch/review, diagnostics, optional realtime/remote
environment, and an end-to-end proof application.

Primary source:

- `docs/macaca-codex-application-capability-gap-research.md`

Hard constraints:

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`
- `macaca/docs/design_patterns.md`

## Superpowers Brainstorm Summary

The research compared four options:

1. Wrap Codex as an external process.
2. Build Codex-specific OS hooks.
3. Build generic Interactive Agent Workbench services.
4. Add a Codex-compatible app-server protocol shell.

The selected architecture is option 3, with option 4 as a shell/gateway surface
after the services exist. Option 1 may be a compatibility adapter only. Option 2
is rejected because it violates the Macaca OS constitution.

## Architecture

The architecture is a service-owned workbench substrate:

```text
Application manifest / app UI / app-owned workflow
  -> SDK / SystemFacade / focused clients
  -> service.interaction + service.app_protocol
  -> service.file + service.process + service.sandbox
  -> service.approval + service.hook + service.config
  -> service.plugin_marketplace + service.mcp + service.skill
  -> service.code_intelligence + service.git + service.review
  -> service.diagnostics + optional realtime/remote environment
  -> service.tool + service.llm + memory/context/task services
  -> EventLog / audit / artifact / telemetry
```

No OS-layer code may branch on a coding application name, Codex product name,
provider name, model name, plugin name, or business workflow. The application
declares capabilities, and services decide availability, policy, resource,
budget, entitlement, and approval through typed contracts.

## Design Patterns

- **Facade:** `SystemFacade` and focused clients for interaction, file,
  process, sandbox, approval, hook, config, plugin, MCP, skill, code, Git,
  review, diagnostics, realtime, and remote environment.
- **Command:** every service operation is a typed command/result DTO with
  trace context.
- **Adapter / Bridge:** JSON-RPC, REST/SSE, stdio, websocket, local filesystem,
  PTY, Docker, SSH, MCP, plugin, Git, code intelligence, and shell surfaces.
- **Strategy:** provider routing, sandbox mode, permission profile, approval
  reviewer, hook selection, analyzer selection, model routing, retry, and
  degradation.
- **Decorator:** trace, policy, resource, entitlement, budget, approval,
  timeout, output redaction, and metering before side effects.
- **State:** thread, turn, item, process, sandbox, approval, hook, plugin,
  MCP, skill, review, realtime, and remote environment lifecycles.
- **Observer:** EventLog, SSE, app protocol notifications, filesystem watch,
  process output, provider health, approval updates, and diagnostics.
- **Memento:** turn/item ledger, snapshots, patches, rollback markers,
  artifacts, config snapshots, approval records, and diagnostic bundles.
- **Specification:** service admission, path policy, network policy, package
  admission, permission profile constraints, managed-only hooks, and optional
  module gates.
- **Abstract Factory:** runtime-host composition roots for providers and
  optional modules.
- **Null Object:** unavailable providers for absent optional services.

## OpenSpec

One umbrella proposal owns the complete support target:

- `openspec/changes/complete-codex-class-application-support/`

The umbrella change contains multiple delta specs so implementation can proceed
service-by-service while still being governed as one complete capability:

- `interaction-ledger`
- `app-protocol-gateway`
- `filesystem-process-sandbox`
- `approval-hook-config`
- `plugin-mcp-skill-lifecycle`
- `code-intelligence-review-diagnostics`
- `codex-class-application-proof`

## Implementation Phases

### Phase 0: Contracts, DTOs, and Service Descriptors

Create provider-neutral DTOs and service descriptors for every service in the
umbrella proposal. Add Null Object clients and SDK focused clients before any
shell integration. No provider implementation should be created in the kernel
or SDK.

Acceptance:

- All new service commands require trace context.
- Every service has descriptor, lifecycle, health, snapshot, and structured
  unavailable behavior.
- DTOs contain sanitized audit refs and artifact refs where payloads may be
  large or sensitive.

### Phase 1: Interaction Ledger and App Protocol Gateway

Implement `service.interaction` with durable Thread/Turn/Item lifecycle and
streaming item events. Implement `service.app_protocol` as a shell/gateway
adapter over focused clients with JSON-RPC, websocket/stdio-compatible framing,
initialization handshake, subscriptions, backpressure, and health probes.

Acceptance:

- Thread start/resume/fork/archive/rollback works.
- Turn start/interrupt/steer works.
- Items stream and persist with replayable event refs.
- The app protocol gateway owns transport adaptation only.

### Phase 2: Filesystem, Process, PTY, and Sandbox

Implement `service.file`, `service.process`, and `service.sandbox` with real
local providers, unavailable providers, and runtime-host factories for future
Docker/SSH/remote providers. Wire file/process/sandbox capabilities into
`service.tool` through provider-backed descriptors.

Acceptance:

- Filesystem read/write/patch/diff/list/metadata/watch operations are policy
  gated and audited.
- Process exec/spawn/stdin/resize/terminate/status/output streaming works.
- Sandbox profiles resolve before process side effects.
- Oversized output and file payloads become artifacts.

### Phase 3: Approval, Hook, Config, Requirements, and LLM Hardening

Implement `service.approval`, `service.hook`, and `service.config`. Harden
`service.llm` with model catalog, provider capability reads, continuation
validation, route diagnostics, and degradation reporting.

Acceptance:

- Approval requests persist, resolve, expire, and audit correctly.
- Pre/post hooks run as decorators, with managed-only policy support.
- Config supports layered reads/writes, requirements, hot reload, permission
  profiles, and feature flags.
- Provider protocol validation catches continuation issues such as missing
  reasoning payloads before app-level failure loops.

### Phase 4: Plugin Marketplace, MCP, and Skill Lifecycle

Complete plugin marketplace lifecycle and operator-grade MCP/skill lifecycle:
install/upgrade/uninstall, auth state, bundled capabilities, MCP status,
resources, OAuth, reload, skill read/config/watch/enablement/provenance.

Acceptance:

- Plugin packages enter only through store/entitlement/policy gates.
- MCP and skill operator diagnostics are structured and sanitized.
- Bundled plugin capabilities register through service descriptors, not shell
  code.

### Phase 5: Code Intelligence, Git, Patch, Review, and Diagnostics

Implement generic code intelligence, Git, patch, review, and diagnostics
services. These services must be usable by coding, migration, QA, compliance,
documentation, and other applications without product-specific branches.

Acceptance:

- Git status/diff/apply-patch/rollback marker operations are audited.
- Code search/symbol context uses provider adapters.
- Review execution produces structured findings and trace refs.
- Diagnostics creates privacy-filtered trace bundles and health summaries.

### Phase 6: Optional Realtime and Remote Environments

Add optional realtime and remote environment service contracts and providers.
Absence must produce structured unavailable states. Presence must not change
base OS semantics.

Acceptance:

- Realtime text/audio and remote exec-server selection are optional modules.
- Remote environment health, workspace roots, and cleanup are traceable.

### Phase 7: Codex-class Application Proof

Build an application-neutral proof application or fixture that declares the
workbench capability set and performs a real coding workflow through Macaca OS
services:

1. Start a thread and turn.
2. Inspect repository files through `service.file`.
3. Search code through `service.code_intelligence`.
4. Apply a patch through `service.git`.
5. Run tests through `service.process` under `service.sandbox`.
6. Invoke MCP/skill tools through `service.tool`.
7. Produce review findings through `service.review`.
8. Stream Thread/Turn/Item, process output, tool lifecycle, approval, and
   diagnostics through `service.app_protocol`.
9. Persist replayable audit, artifacts, and diagnostics.

Acceptance:

- The proof uses no application-specific OS code.
- The same service set could support non-coding applications.
- Live/API validation proves end-to-end execution, not only catalog visibility.

## Validation Strategy

- Run `openspec validate complete-codex-class-application-support --strict`.
- Add contract tests for every service DTO and Null Object client.
- Add service runtime lifecycle/health/snapshot tests.
- Add policy-before-side-effect tests for file/process/sandbox/git/plugin/MCP.
- Add dependency-boundary tests to prevent kernel, SDK, Web, CLI, or frontend
  from becoming semantic owners.
- Add audit replay tests for thread/turn/item, file, process, approval, hook,
  plugin, MCP, skill, review, and diagnostics.
- Add live application-neutral proof using real local file/process/sandbox
  providers.

## Completion Definition

The proposal is complete only when Macaca can truthfully support a
Codex-class coding application as a normal application with no OS-layer
hardcoded workflow. Completion requires:

- All services implemented with real local providers or explicit unavailable
  optional providers.
- SDK/SystemFacade focused clients available.
- Shells and app protocol gateway remain adapters.
- Provider absence is structured unavailable, not crash, hang, silent fallback,
  or fake success.
- Trace, audit, logs, artifacts, and diagnostics are bounded and sanitized.
- End-to-end proof demonstrates real multi-service coding workflow.
