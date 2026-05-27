# Change: Complete Codex-class Application Support

## Why

`docs/macaca-codex-application-capability-gap-research.md` shows that Macaca
has the correct OS direction but does not yet provide the full generic
interactive-agent substrate needed for a Codex-class coding application. Current
capabilities cover parts of service runtime, application framework, tools,
MCP, skills, memory/context, scheduling, and observability, but a production
coding workbench still needs durable Thread/Turn/Item interaction state,
bidirectional app protocol, provider-backed filesystem/process/sandbox,
approval/hook/config lifecycle, plugin marketplace, MCP/skill operator
lifecycle, code intelligence, Git/patch/review, diagnostics, optional
realtime/remote environments, and an end-to-end proof.

This change implements the complete generic support surface. It intentionally
does not implement Codex as OS behavior and does not hardcode a coding
application workflow. A Codex-like application must remain an ordinary Macaca
application that declares and composes capabilities through service boundaries.

## What Changes

- Add `service.interaction` for durable Thread/Turn/Item lifecycle, replay,
  fork/archive/rollback, turn steering/interruption, and item notifications.
- Add `service.app_protocol` as a shell/gateway adapter for bidirectional
  JSON-RPC/stdout/websocket/unix-socket style clients over focused SDK clients.
- Add provider-backed `service.file`, `service.process`, and `service.sandbox`
  with local providers, structured unavailable optional providers, policy,
  resource gates, artifacts, and audit.
- Add `service.approval`, `service.hook`, and `service.config` for approval
  queues, hook decorators, layered config, admin requirements, permission
  profiles, feature flags, and hot reload.
- Harden `service.llm` with model catalog, provider capability reads,
  continuation protocol validation, degradation diagnostics, and budget
  surfaces.
- Complete plugin marketplace, MCP, and skill operator lifecycle through
  service-owned commands, diagnostics, auth state, watched changes, config, and
  capability registration.
- Add generic code intelligence, Git/patch, review, diagnostics, optional
  realtime, and remote-environment services.
- Extend SDK/SystemFacade focused clients and Null Object behavior for all new
  services.
- Wire service-backed capabilities into `service.tool` planning and invocation
  without stealing ownership from provider services.
- Add Web/CLI/frontend/app-protocol adapters that render and subscribe to state
  while keeping semantics in services.
- Add end-to-end application-neutral proof that runs a real coding workflow
  through Macaca services without application-specific OS branches.

## Impact

- Affected specs:
  - `interaction-ledger`
  - `app-protocol-gateway`
  - `filesystem-process-sandbox`
  - `approval-hook-config`
  - `plugin-mcp-skill-lifecycle`
  - `code-intelligence-review-diagnostics`
  - `codex-class-application-proof`
- Affected code:
  - `macaca/crates/foundation/macaca-proto`
  - `macaca/crates/foundation/macaca-ipc`
  - `macaca/crates/kernel/macaca-kernel`
  - `macaca/crates/services/*`
  - `macaca/crates/runtime/macaca-runtime-host`
  - `macaca/crates/facade/macaca-sdk`
  - `macaca/crates/application/macaca-app`
  - `macaca/crates/shells/macaca-web`
  - `macaca/crates/shells/macaca-cli`
  - `frontend/`
  - `macaca/crates/tests/macaca-integration-tests`
- Governance:
  - Must preserve `macaca-os-architecture-governance.md`.
  - Must preserve `macaca-os-microkernel-boundaries.md`.
  - Must preserve `macaca-os-serviceization-allowlist.md`.
  - Must not add application-specific OS code.
  - GitNexus CRITICAL/HIGH findings are recorded as notes for this refactor
    per user instruction, not treated as blockers.
