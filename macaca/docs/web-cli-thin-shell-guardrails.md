# Web / CLI Thin Shell Guardrails

Route C Phase 12 makes Web, frontend, and CLI presentation shells. They adapt user-facing transports into typed system commands and delegate semantics to SDK/Application/Kernel/service facades.

## Web Shell Rules

- Web routes own HTTP parsing, status mapping, SSE transport, and response formatting.
- Web routes should construct typed SDK commands rather than implement task, session, trace, package, approval, payment, Web3, EVM, plugin, or entitlement semantics.
- Migrated routes must preserve existing JSON shape until frontend callers are explicitly migrated.
- Route handlers must log scope validation, command construction, facade execution, success, and structured rejection.
- New Web shell code must not hardcode application names, workflow names, driver names, gateway names, model names, provider names, chain names, package names, or business routes.

## CLI Shell Rules

- CLI commands own terminal parsing, terminal formatting, process startup, and exit behavior.
- CLI system inspection commands should use SDK facades or typed lower-layer facades.
- CLI must not depend on `macaca-web` internals for system inspection semantics.
- Deprecated direct helpers may remain as compatibility shims, but new command handlers should use facade-backed paths.

## Frontend Shell Rules

- Frontend renders chat, trace, task board, approvals, packages, and GenUI as generic shell surfaces.
- GenUI rendering must dispatch by schema/component/event kind, not by application-specific names.
- When no GenUI surface exists, the default chat/trace shell remains available.

## Trace and Audit Rules

- Web/CLI shell command boundaries must emit structured logs for scope validation, command construction, facade execution, success, rejection, and failure.
- Trace/event replay must stay session-scoped and cursor-based.
- Logs must not include secrets, private keys, provider credentials, raw payment credentials, raw encrypted package contents, raw unbounded user input, or provider secrets.
