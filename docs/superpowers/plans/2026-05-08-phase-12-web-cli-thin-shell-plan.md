# Phase 12 Web / CLI Thin Shell Implementation Plan

## Scope

Implement Route C Phase 12 as a gradual Web/CLI thin shell migration. The phase introduces a shell-facing SDK system facade, migrates low-risk Web routes and CLI commands through command adapters, and documents/preserves frontend shell constraints. It must not rewrite the whole Web server, change `/api/chat/v2` behavior, break SSE, or move presentation semantics into kernel.

## Architecture Choice

Use an SDK-first facade:

- `macaca-sdk`: shell-facing system facade and command/result contracts.
- `macaca-web`: HTTP/SSE/GenUI adapters that convert requests into SDK commands.
- `macaca-cli`: terminal command handlers that delegate to SDK facade commands.
- frontend: generic shell/GenUI/trace mount rules only; no application-specific UI hardcode.

## Required Design Patterns

- Facade: stable shell-facing `SystemFacade`.
- Command: HTTP and CLI inputs become typed system commands.
- Adapter/Bridge: Web routes, SSE, CLI, and frontend shells adapt transports only.
- Observer: Web subscribes to trace/event streams without defining trace semantics.
- Visitor: frontend renders GenUI/trace/package metadata by schema kind.
- Specification: route/session scope, permissions, package metadata, and compatibility validation.
- Memento: session snapshots, trace replay cursors, and task board views remain replayable data.

## Implementation Slices

### Slice 12.1: SDK system facade

- Add `macaca/crates/macaca-sdk/src/system_facade.rs`.
- Define shell-facing commands/results for task board query, session event query, service inspection, package inspection, approval decision, and trace tail intent.
- Keep methods concrete and typed; do not create a generic stringly RPC dumping ground.
- Add no-network tests with mock facade data.

### Slice 12.2: Web route command adapter

- Add or extend `macaca/crates/macaca-web/src/shell.rs`.
- Migrate one low-risk route first, preferably task board or session events, while preserving response JSON shape.
- Route handlers should validate request scope, log command execution, call SDK facade, and return data.
- Existing frontend calls should not need to change for the first slice.

### Slice 12.3: Trace/SSE subscription thin shell

- Define a shell-facing trace subscription/read model.
- Web should subscribe to event/trace services and render/forward events; it must not invent core trace semantics.
- Preserve real-time and replay behavior without duplicate events.

### Slice 12.4: GenUI shell mount guardrails

- Document and optionally add frontend shell contracts for generic GenUI mount.
- Default remains chat/trace shell when no GenUI surface exists.
- GenUI rendering must dispatch by schema/component kind, not by app/workflow/provider names.

### Slice 12.5: CLI facade migration

- Add CLI command adapter ownership rules.
- Migrate read-only commands such as service inspect/status/app list/session inspect/trace tail through SDK facade where current commands exist or are added.
- Keep deprecated compatibility helpers present until all callers migrate.

## Files Expected

- New: `macaca/crates/macaca-sdk/src/system_facade.rs`
- Possible modify: `macaca/crates/macaca-sdk/src/lib.rs`
- New: `macaca/crates/macaca-web/src/shell.rs`
- Possible modify: `macaca/crates/macaca-web/src/routes.rs`
- Possible modify: `macaca/crates/macaca-web/src/sse.rs`
- Possible modify: `macaca/crates/macaca-cli/src/command_handlers.rs`
- Possible modify: `macaca/crates/macaca-cli/src/commands.rs`
- Possible frontend docs or shell mount files under frontend.
- OpenSpec: `openspec/changes/add-web-cli-thin-shell-v0/`

## Mandatory Constraints

- Do not rewrite the whole `macaca-web`.
- Do not change `/api/chat/v2` wire behavior in the first slice.
- Do not let Web/CLI define session, task, trace, package, service, payment, Web3, EVM, plugin, or entitlement semantics.
- Do not move presentation shell logic into kernel.
- Do not hardcode application name, workflow name, driver name, gateway name, model name, provider name, chain name, package name, or business route.
- Do not add frontend application-specific UI branches.
- All new Rust code must include detailed English comments and structured `tracing` logs at key execution nodes.
- Keep Rust files below 500 lines.

## Verification

- `openspec validate add-web-cli-thin-shell-v0 --strict`
- `cargo test -p macaca-sdk`
- `cargo test -p macaca-web`
- `cargo check -p macaca-cli`
- `cargo test -p macaca-integration-tests --test route_c_baseline`
- frontend lint/typecheck if frontend files change: `cd frontend && npm run lint && npx tsc --noEmit`
- hardcode scan over new shell/facade files
- `npx gitnexus detect-changes --repo agent`

## Commit Plan

After approval and implementation:

- Commit 1: SDK system facade contracts and tests.
- Commit 2: one Web route command adapter migration and tests.
- Commit 3: trace/SSE shell guardrails, CLI facade migration, frontend shell docs or mount guardrails.

Do not commit unrelated dirty files.
