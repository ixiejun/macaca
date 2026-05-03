# Design

## Context

`macaca-gateway` is a low-level external protocol entry crate. It currently depends only on `macaca-proto` plus infrastructure crates and is consumed mainly by `macaca-cli` and integration tests. It must not depend on `macaca-web`, `macaca-kernel`, `macaca-app`, app names, workflow names, or driver names.

Current implementation risks:

- `telegram.rs` is over 500 lines and mixes parsing, long polling, sending, splitting, and tests.
- `ImAdapter` mixes lifecycle and sending with event dispatch.
- CLI manually constructs Telegram/Discord concrete adapters.
- `Gateway` is a lifecycle manager, not yet a gateway mediator boundary.

## Goals / Non-Goals

- Goal: introduce gateway primitives through additive-first APIs.
- Goal: keep behavior 1:1 compatible with current tests.
- Goal: make deprecated legacy interfaces easy to find without deleting them.
- Goal: keep every gateway source file under 500 lines.
- Non-goal: integrate gateway with web sessions or chat dispatch.
- Non-goal: implement real Discord API support.

## Pattern Mapping

- Adapter: platform raw messages convert into gateway-neutral primitives.
- Bridge: `GatewayTransport` separates platform transport from gateway orchestration.
- Mediator: `GatewayMediator` coordinates message handling and replies.
- Strategy: `GatewayReplyFormatter` handles platform-specific reply formatting.
- Factory/Builder: `GatewayBuilder` constructs gateway instances from config.

## Decisions

### Decision 1: Additive-first public API

New primitives are introduced beside existing APIs. Existing `ImAdapter`, `EventHandler`, and `Gateway` lifecycle APIs remain available but are marked deprecated after equivalent new primitives exist. Production CLI startup migrates to `GatewayBuilder`; compatibility tests and internal bridge code may keep local `#[allow(deprecated)]`.

Alternative considered: replace `ImAdapter` immediately with `GatewayTransport`. Rejected because it would force simultaneous updates to CLI and integration tests and increase regression risk.

### Decision 2: Gateway remains independent from upper layers

`GatewayMediator` dispatches through a trait boundary and current `EventHandler`; it does not call web routes, kernel executors, app runtimes, or application-specific workflows.

Alternative considered: wire mediator directly into `chat_v2`. Rejected because gateway is a low-level protocol crate and should not import web concerns.

### Decision 3: Telegram is split by responsibility

Telegram parsing, formatting/splitting, and adapter runtime are separated into focused modules. Public `TelegramAdapter` re-export remains stable.

Alternative considered: keep one file and only add abstractions. Rejected because it violates the project file-size constraint.

## Risks / Trade-offs

- Risk: temporary API duplication between legacy adapter traits and new transport/mediator primitives.
  Mitigation: deprecate old interfaces and add tests for the new path.

- Risk: Telegram formatting changes accidentally.
  Mitigation: preserve current newline-preferred splitting tests and route the formatter through the same algorithm.

- Risk: builder config behavior changes disabled adapter handling.
  Mitigation: builder tests cover disabled, Telegram-only, Discord-only, and both-platform combinations.

## Migration Plan

1. Add OpenSpec contract and validate.
2. Run GitNexus impact for gateway symbols.
3. Add neutral message primitives.
4. Add transport boundary and adapter implementations.
5. Add mediator boundary.
6. Add reply formatting strategy.
7. Split Telegram module under 500 lines.
8. Add config-driven builder.
9. Migrate CLI gateway startup to the builder.
10. Mark legacy interfaces deprecated.
11. Run gateway tests, integration tests, cargo check, OpenSpec validation, and GitNexus detect changes.

## Open Questions

None for this slice. Web/session/chat dispatch integration remains intentionally left for a separate proposal.
