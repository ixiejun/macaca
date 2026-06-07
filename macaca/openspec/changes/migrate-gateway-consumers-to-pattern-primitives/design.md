## Context

The gateway design-pattern refactor introduced additive primitives:

- `GatewayBuilder`
- `GatewayMediator`
- `GatewayEventSink`
- `GatewayTransport`
- `GatewayInboundMessage`
- `GatewayReply`
- reply formatting strategies

Direct Cargo consumers of `macaca-gateway` are currently limited to `macaca-cli` and `macaca-integration-tests`. `macaca-cli` is the production consumer and should use the builder path. Integration tests should exercise the new primitives without relying on deprecated compatibility APIs.

## Goals

- Ensure upper crates do not call deprecated gateway lifecycle APIs directly.
- Keep `macaca-cli` gateway startup on `GatewayBuilder`.
- Add cross-crate tests for builder, mediator, and transport primitives.
- Preserve deprecated API definitions inside `macaca-gateway` without deleting them.
- Keep gateway independent from web, kernel, app, workflow, driver, and application-specific names.

## Non-Goals

- Do not remove deprecated gateway APIs.
- Do not replace gateway internal compatibility bridge in this change.
- Do not add new gateway platforms.
- Do not connect gateway to chat_v2, sessions, EventLog, kernel, or application runtimes.

## Decisions

- `macaca-cli` remains the only production gateway consumer and uses `GatewayBuilder`.
- `macaca-integration-tests` moves away from legacy `Gateway` / `ImAdapter` / `EventHandler` calls.
- Deprecated usage checks distinguish upper consumers from `macaca-gateway` internals, where compatibility definitions and bridge code are still allowed.
- Compatibility for old APIs is protected by `macaca-gateway` crate tests, not upper integration tests.

## Risks / Trade-offs

- Removing legacy integration-test usage reduces cross-crate coverage for deprecated APIs.
  - Mitigation: deprecated APIs remain covered inside `macaca-gateway` tests and remain public.
- `GatewayBuilder` still uses deprecated internals while the internal bridge exists.
  - Mitigation: this is contained inside `macaca-gateway`; upper consumers use non-deprecated builder APIs.
- GatewayBuilder and GatewayMediator are currently uncommitted, so GitNexus cannot impact them by symbol name yet.
  - Mitigation: use grep for direct consumers now and rerun GitNexus detect changes after implementation.
