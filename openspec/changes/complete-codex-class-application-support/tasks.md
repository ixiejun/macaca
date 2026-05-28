## 0. Governance and Scope Control

- [x] 0.1 Re-read `docs/macaca-codex-application-capability-gap-research.md`
  and confirm every capability gap is mapped to a service, plugin, optional
  module, application-framework contract, SDK client, or shell adapter.
- [x] 0.2 Re-read `macaca/docs/macaca-os-architecture-governance.md`,
  `macaca/docs/macaca-os-microkernel-boundaries.md`, and
  `macaca/docs/macaca-os-serviceization-allowlist.md` before implementation.
- [x] 0.3 Run GitNexus impact analysis before editing symbols; record
  CRITICAL/HIGH warnings as notes per user instruction.
- [x] 0.4 Define a completion ledger that maps each research-report gap to
  tasks in this checklist and fails completion when any gap is descriptor-only.
- [x] 0.5 Add or update boundary tests that reject application-name, workflow
  name, provider-name, model-name, gateway-name, chain-name, or business-domain
  branches below the application layer.

## 1. Foundation Contracts and SDK Facades

- [x] 1.1 Add provider-neutral DTO modules in `macaca-proto` for
  interaction ledger, app protocol, file, process, sandbox, approval, hook,
  config, plugin marketplace, code intelligence, Git, review, diagnostics,
  realtime, and remote environment.
- [x] 1.2 Add command constants and result enums for unavailable, unsupported,
  denied, failed, pending approval, cancelled, timeout, artifact-backed, and
  completed states.
- [x] 1.3 Add service descriptors with lifecycle, health, snapshot, command
  list, event schema, and audit schema for every new or upgraded service.
- [x] 1.4 Add trace-required command wrappers so every public service command
  carries application id, session id, optional task/turn/thread refs, and
  `TraceContext`.
- [x] 1.5 Add artifact-ref DTOs and bounded summary DTOs for file contents,
  process output, item payloads, diagnostics bundles, review evidence, and
  provider payloads.
- [x] 1.6 Add focused SDK clients for each service.
- [x] 1.7 Add Null Object SDK clients returning structured unavailable states
  without panics, fake success, or silent fallback.
- [x] 1.8 Add SystemFacade entrypoints that expose the focused clients without
  constructing concrete providers.
- [x] 1.9 Add English comments to non-obvious DTOs and clients explaining
  ownership, trace requirements, redaction rules, and operating principles.
- [x] 1.10 Add contract tests for DTO serialization, command names, lifecycle
  descriptors, health snapshots, and Null Object behavior.

## 2. `service.interaction`: Thread / Turn / Item Ledger

- [x] 2.1 Create `service.interaction` provider crate or runtime-host provider
  wrapper with descriptor, lifecycle, health, snapshot, command dispatch, and
  structured unavailable provider.
- [x] 2.2 Implement `interaction.thread.start`, `resume`, `fork`, `archive`,
  `unarchive`, `rollback`, `list`, `read`, and loaded-thread listing.
- [x] 2.3 Implement `interaction.turn.start`, `interrupt`, `steer`,
  `complete`, `fail`, and turn history listing.
- [x] 2.4 Implement `interaction.item.append`, `complete`, `fail`, `list`, and
  item watch/subscribe with bounded payloads.
- [x] 2.5 Persist Thread/Turn/Item records in a replayable store without using
  shell memory as the source of truth.
- [x] 2.6 Store oversized or sensitive item payloads as artifact refs.
- [x] 2.7 Emit EventLog and audit events for thread, turn, and item lifecycle.
- [x] 2.8 Add replay tests for resume, fork, rollback, interrupted turn,
  completed turn, failed turn, and artifact-backed item payloads.
- [x] 2.9 Add compatibility adapters from existing session/event-log paths to
  the new interaction service without regressing `/api/chat/v2`.
- [x] 2.10 Add shell-facing read/list APIs through SDK clients only.

## 3. `service.app_protocol`: Bidirectional Protocol Gateway

- [x] 3.1 Create `service.app_protocol` descriptor, lifecycle, health,
  snapshot, connection state, and unavailable provider.
- [x] 3.2 Implement initialize/initialized handshake with client metadata,
  protocol version, capability negotiation, and notification opt-out.
- [x] 3.3 Implement subscription create/close and thread/app/service event
  routing through focused clients.
- [x] 3.4 Implement JSON-RPC framing for websocket-compatible transport.
- [x] 3.5 Implement stdio/unix-socket-compatible transport adapters where the
  runtime supports them; otherwise expose structured unavailable diagnostics.
- [x] 3.6 Implement bounded ingress/outbound queues, overload error, retryable
  reason code, and backpressure logs.
- [x] 3.7 Implement health probes and connection lifecycle diagnostics.
- [x] 3.8 Translate Thread/Turn/Item, process output, filesystem changed,
  approval, hook, tool, review, diagnostics, MCP, skill, and plugin events into
  typed protocol notifications.
- [x] 3.9 Prove the gateway does not own interaction, file, process, sandbox,
  approval, plugin, MCP, skill, tool, Git, review, or diagnostics semantics.
- [x] 3.10 Add protocol tests for initialization gating, duplicate
  initialization, overload, subscription close, event ordering, and redaction.

## 4. `service.file`: Filesystem Provider

- [x] 4.1 Create `service.file` descriptor, lifecycle, health, snapshot,
  command dispatch, local provider, mock provider, and unavailable provider.
- [x] 4.2 Implement `file.read` with workspace root policy, symlink policy,
  binary detection, size budget, artifact fallback, and sanitized audit.
- [x] 4.3 Implement `file.write` with pre-write memento, path policy, approval
  integration, artifact handling, and audit.
- [x] 4.4 Implement `file.patch` with structured patch input, conflict
  diagnostics, pre/post hashes, and rollback refs.
- [x] 4.5 Implement `file.copy`, `file.remove`, `file.metadata`,
  `file.directory.list`, and bounded result summaries.
- [x] 4.6 Implement `file.watch` and `file.unwatch` with stable watch ids and
  bounded changed notifications.
- [x] 4.7 Wire file descriptors into `service.tool` planning and invocation as
  service-owned providers.
- [x] 4.8 Add tests for path traversal, symlink denial, read-only denial,
  write approval, oversized file artifacts, watch notification, and audit
  replay.

## 5. `service.process`: Command, PTY, and Background Processes

- [x] 5.1 Create `service.process` descriptor, lifecycle, health, snapshot,
  local provider, mock provider, and unavailable provider.
- [x] 5.2 Implement `process.exec` for bounded command execution with sandbox
  profile resolution before spawn.
- [x] 5.3 Implement `process.spawn` for long-running/background processes with
  process handles and lifecycle events.
- [x] 5.4 Implement PTY allocation, `stdin.write`, `pty.resize`, and
  `process.terminate`.
- [x] 5.5 Implement output subscription and base64/binary-safe output deltas
  with inline budget and artifact refs.
- [x] 5.6 Implement background process cleanup by thread/session/application
  scope.
- [x] 5.7 Add command hash, executable summary, cwd scope, sandbox ref,
  resource lease refs, exit status, duration, output byte counts, and audit
  refs to every process record.
- [x] 5.8 Wire process/shell descriptors into `service.tool` planning and
  invocation.
- [x] 5.9 Add tests for policy-before-spawn, denied command no-op,
  cancellation, stdin, PTY resize, timeout, output truncation, artifact output,
  background cleanup, and audit replay.

## 6. `service.sandbox`: Permission Profiles and Runtime Environments

- [x] 6.1 Create `service.sandbox` descriptor, lifecycle, health, snapshot,
  local provider, mock provider, and unavailable provider.
- [x] 6.2 Implement permission profile catalog and resolution for read-only,
  workspace-write, full-access, network modes, and remote environment modes.
- [x] 6.3 Implement sandbox environment prepare/health/cleanup with resource
  leases and trace refs.
- [x] 6.4 Implement path, network, environment-variable, workspace-root, and
  write-scope policy explanation.
- [x] 6.5 Add Docker, SSH, OS-specific, browser, and WASM sandbox provider
  seams with explicit unavailable states when absent.
- [x] 6.6 Connect `service.process`, `service.file`, `service.tool`, and
  application manifests to sandbox profile resolution without provider-name
  branches.
- [x] 6.7 Add tests for unavailable optional providers, cleanup after
  cancellation, network denial, write-scope denial, and resource release.

## 7. `service.approval`: Approval and Guardian Flow

- [x] 7.1 Create `service.approval` descriptor, lifecycle, health, snapshot,
  local provider, mock provider, and unavailable provider.
- [x] 7.2 Implement approval request create/list/read/resolve/cancel/expire.
- [x] 7.3 Implement reviewer policy strategy and approval profile resolution.
- [x] 7.4 Integrate approval decorators before file write, process spawn, Git
  patch, plugin install, MCP auth/tool call, remote environment, and other
  privileged side effects.
- [x] 7.5 Emit approval pending/resolved/expired/cancelled events for shells.
- [x] 7.6 Persist sanitized approval audit with action summary, side-effect
  class, reviewer class, decision, reason code, and trace refs.
- [x] 7.7 Add tests proving shells render/submit decisions but do not own
  approval policy.

## 8. `service.hook`: Managed Lifecycle Hooks

- [x] 8.1 Create `service.hook` descriptor, lifecycle, health, snapshot, local
  provider, mock provider, and unavailable provider.
- [x] 8.2 Implement hook catalog and hook policy resolution by application,
  session, thread, turn, tool family, and command scope.
- [x] 8.3 Implement pre-tool hooks that can continue, rewrite bounded input, or
  block before provider dispatch.
- [x] 8.4 Implement post-tool hooks that can add bounded additional context,
  request stop, or replace model-visible tool output.
- [x] 8.5 Implement session/turn lifecycle hooks.
- [x] 8.6 Implement managed-only hook requirements and ignore user/project/
  session hooks when required.
- [x] 8.7 Add script/plugin/WASM/remote hook adapter seams with unavailable
  behavior where providers are absent.
- [x] 8.8 Add tests for hook ordering, block before side effect, managed-only
  filtering, bounded feedback, audit, and event emission.

## 9. `service.config` and Requirements

- [x] 9.1 Create `service.config` descriptor, lifecycle, health, snapshot,
  local provider, mock provider, and unavailable provider.
- [x] 9.2 Implement layered config read from default, user, project,
  application, session, requirements, and managed sources.
- [x] 9.3 Implement single value write and batch write with atomicity,
  validation, redaction, and optional hot reload.
- [x] 9.4 Implement config schema read and generated schema tests.
- [x] 9.5 Implement requirements read for allowed approval policies, sandbox
  modes, web/network modes, permissions, managed hook policy, residency,
  feature requirements, and network constraints.
- [x] 9.6 Implement permission profile list by cwd/application scope.
- [x] 9.7 Implement feature flag list and runtime enablement patching.
- [x] 9.8 Add tests for secret redaction, requirement precedence, hot reload,
  invalid config, and permission profile constraints.

## 10. `service.llm` Hardening

- [ ] 10.1 Add model catalog commands for list, provider capabilities, route
  resolution, budget status, and degradation explanation.
- [ ] 10.2 Add provider protocol metadata contracts for reasoning,
  tool-call/tool-result continuation, service tiers, and retry policy.
- [ ] 10.3 Implement continuation validation before dispatching tool-result
  follow-up calls to providers.
- [ ] 10.4 Fix provider continuation paths that currently fail on missing
  provider-specific reasoning continuation metadata.
- [ ] 10.5 Add structured diagnostics for provider unavailable, unsupported
  model, protocol mismatch, budget denied, rate limited, and degradation.
- [ ] 10.6 Add tests reproducing the DeepSeek thinking-mode continuation error
  and proving structured validation/fix behavior.

## 11. Plugin Marketplace Lifecycle

- [ ] 11.1 Create or complete `service.plugin_marketplace` descriptor,
  lifecycle, health, snapshot, local provider, mock provider, and unavailable
  provider.
- [ ] 11.2 Implement marketplace add/remove/upgrade with source policy,
  signature, version, and entitlement checks.
- [ ] 11.3 Implement plugin list/read/install/uninstall/enable/disable/auth
  status.
- [ ] 11.4 Implement bundled capability registration for services, tools,
  skills, MCP servers, hooks, apps, and app UI metadata.
- [ ] 11.5 Integrate store/entitlement/license/metering before plugin
  installation and capability activation.
- [ ] 11.6 Add plugin rollback and uninstall cleanup records.
- [ ] 11.7 Add tests for unavailable marketplace, denied entitlement,
  malformed manifest, disabled-by-admin plugin, bundled capability registration,
  and audit replay.

## 12. MCP Operator Lifecycle

- [ ] 12.1 Upgrade `service.mcp` descriptor and commands for server status,
  reload, resource read, tool call, OAuth login/status, diagnostics snapshot,
  and watched changes.
- [ ] 12.2 Implement per-thread MCP exposure refresh on next active turn after
  reload.
- [ ] 12.3 Implement OAuth auth-required, login-started, login-completed,
  failed, and unavailable states.
- [ ] 12.4 Implement resource/resource-template listing and bounded resource
  read.
- [ ] 12.5 Ensure MCP tool calls continue to route through `service.tool` when
  model-invoked.
- [ ] 12.6 Add tests for reload, OAuth-required denial, resource read, tool
  call audit, status snapshots, and plugin-provided MCP servers.

## 13. Skill Operator Lifecycle

- [ ] 13.1 Upgrade `service.skill` descriptor and commands for catalog list,
  markdown read, config write, watch/unwatch, changed events, enablement
  changes, and provenance audit.
- [ ] 13.2 Implement app-scoped, workspace-scoped, user-scoped, managed, and
  plugin-provided skill source handling.
- [ ] 13.3 Implement skill config persistence with policy and redaction.
- [ ] 13.4 Implement skill watch notifications and service-owned visibility
  refresh for context/tool planning.
- [ ] 13.5 Add tests for source precedence, enablement policy, watch changes,
  markdown read bounds, config redaction, and provenance audit.

## 14. Code Intelligence, Git, Patch, and Review

- [ ] 14.1 Create `service.code_intelligence` with descriptor, lifecycle,
  health, snapshot, local/search provider, mock provider, and unavailable
  provider.
- [ ] 14.2 Implement code search, symbol context, file reference discovery, and
  analyzer diagnostics with bounded snippets and provider health diagnostics.
- [ ] 14.3 Create `service.git` with descriptor, lifecycle, health, snapshot,
  local Git provider, mock provider, and unavailable provider.
- [ ] 14.4 Implement git status, diff, apply patch, rollback marker, path
  policy, pre-change memento, post-change hash, and patch provenance.
- [ ] 14.5 Create `service.review` with descriptor, lifecycle, health,
  snapshot, local provider, mock provider, and unavailable provider.
- [ ] 14.6 Implement review start/progress/result, structured findings,
  severity, location, rationale, evidence refs, and artifact-backed review
  payloads.
- [ ] 14.7 Wire code/Git/review descriptors into application manifests,
  service.tool where appropriate, and SDK focused clients.
- [ ] 14.8 Add tests for path denial, patch conflict, rollback marker replay,
  analyzer unavailable, structured review findings, and audit replay.

## 15. Diagnostics, Feedback, Realtime, and Remote Environment

- [ ] 15.1 Create `service.diagnostics` with descriptor, lifecycle, health,
  snapshot, local provider, mock provider, and unavailable provider.
- [ ] 15.2 Implement diagnostics snapshot, feedback upload, trace bundle, health
  summary, redaction profile, and bounded bundle artifact refs.
- [ ] 15.3 Add diagnostics sources for interaction, file, process, sandbox,
  approval, hooks, config, plugins, MCP, skills, tools, LLM, Git, review, and
  app protocol.
- [ ] 15.4 Create optional `service.realtime` contract and unavailable provider;
  add text/audio/WebRTC provider seams without making realtime a base
  dependency.
- [ ] 15.5 Create optional `service.remote_environment` contract and unavailable
  provider; add remote exec-server registration, health, workspace roots,
  cleanup, and selection seams.
- [ ] 15.6 Add tests proving diagnostics redaction, optional provider absence,
  remote health diagnostics, and no base OS dependency on optional modules.

## 16. Application Framework and Manifest Integration

- [ ] 16.1 Extend application manifests to declare workbench capabilities,
  permission profiles, tool families, service dependencies, optional provider
  requirements, plugin dependencies, MCP dependencies, skill bundles, and event
  subscriptions.
- [ ] 16.2 Add manifest admission checks that validate capabilities without
  hardcoding application names or coding workflows.
- [ ] 16.3 Add application lifecycle integration so declared capabilities are
  projected into service clients, context, tool planning, and app protocol
  subscriptions.
- [ ] 16.4 Add GenUI/app-owned UI metadata support for workbench surfaces without
  making frontend own semantics.
- [ ] 16.5 Add tests for YAML, WASM, GenUI, headless, and workbench-style
  applications declaring the same generic capabilities.

## 17. Shell, Web, CLI, and Frontend Adapters

- [ ] 17.1 Add Web routes for interaction, app protocol diagnostics, file,
  process, sandbox, approval, hooks, config, plugin, MCP, skill, code, Git,
  review, diagnostics, realtime, and remote environment through focused
  clients only.
- [ ] 17.2 Add CLI commands for operator diagnostics and local testing through
  focused clients only.
- [ ] 17.3 Add frontend views for Thread/Turn/Item streams, process output,
  file changes, approvals, hooks, plugin/MCP/skill status, review findings,
  diagnostics, and provider health.
- [ ] 17.4 Ensure frontend remains a renderer and never owns policy, approval,
  tool routing, plugin lifecycle, file/process/sandbox semantics, or coding
  workflow logic.
- [ ] 17.5 Add UI/API tests proving shell surfaces degrade gracefully when
  optional providers are unavailable.

## 18. Tool Capability Plane Integration

- [ ] 18.1 Add descriptor contributors from file, process, sandbox, approval,
  hook, plugin, MCP, skill, code intelligence, Git, review, diagnostics,
  realtime, and remote environment services.
- [ ] 18.2 Update `tool.catalog.plan` to surface visible/hidden diagnostics for
  workbench capabilities.
- [ ] 18.3 Update `tool.toolset.resolve` for workbench toolsets without
  application-specific branches.
- [ ] 18.4 Update `tool.invoke` routing to call owning services for all new
  provider-backed families.
- [ ] 18.5 Add policy/resource/approval/artifact/audit coverage for each new
  tool route.
- [ ] 18.6 Add tests proving no descriptor-only route is considered complete
  unless invocation works or returns structured unavailable.

## 19. Codex-class Application-neutral Proof

- [ ] 19.1 Add a proof fixture application manifest declaring the full
  workbench capability set.
- [ ] 19.2 Start a thread and turn through `service.interaction`.
- [ ] 19.3 Inspect repository files through `service.file`.
- [ ] 19.4 Search code through `service.code_intelligence`.
- [ ] 19.5 Apply a patch through `service.git`.
- [ ] 19.6 Run tests through `service.process` under `service.sandbox`.
- [ ] 19.7 Trigger and resolve an approval through `service.approval`.
- [ ] 19.8 Run pre/post hooks through `service.hook`.
- [ ] 19.9 Invoke at least one MCP or skill tool through `service.tool`.
- [ ] 19.10 Produce structured review findings through `service.review`.
- [ ] 19.11 Produce diagnostics through `service.diagnostics`.
- [ ] 19.12 Stream all Thread/Turn/Item, process output, file change, tool,
  approval, hook, review, and diagnostics events through `service.app_protocol`.
- [ ] 19.13 Replay audit evidence and artifact refs for the workflow.
- [ ] 19.14 Prove the workflow contains no OS-layer application-specific
  branches.

## 20. Validation and Completion Gates

- [ ] 20.1 Run `openspec validate complete-codex-class-application-support --strict`.
- [ ] 20.2 Run targeted Rust tests for every touched crate.
- [ ] 20.3 Run dependency-boundary and serviceization gates.
- [ ] 20.4 Run audit replay tests for every new service family.
- [ ] 20.5 Run `/api/chat/v2` regression.
- [ ] 20.6 Run YAML/WASM/GenUI application regressions.
- [ ] 20.7 Run industrial tools regressions.
- [ ] 20.8 Run frontend lint/build if frontend code changes.
- [ ] 20.9 Run live API proof for the application-neutral Codex-class workflow.
- [ ] 20.10 Run GitNexus detect changes before commit and record CRITICAL/HIGH
  warnings as notes.
- [ ] 20.11 Mark this proposal complete only when all services are real
  provider-backed or explicitly unavailable optional providers, all shell
  adapters remain thin, and the full proof passes.
