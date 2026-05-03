# Change: Refactor macaca-gateway with Design Pattern Primitives

## Why

`macaca-gateway` is the external protocol entry layer for Agent OS. It currently mixes platform adapter lifecycle, platform parsing, reply formatting, and gateway coordination in a small set of concrete types. `telegram.rs` also exceeds the project file-size limit and combines polling, parsing, sending, splitting, and tests.

This change introduces additive-first gateway primitives so future Telegram, Discord, Slack, email, or custom gateway surfaces can be added without hardcoding platform logic into CLI, web, kernel, or application code.

## What Changes

- Add platform-neutral inbound/outbound gateway message primitives.
- Add `GatewayTransport` as the transport boundary while keeping legacy `ImAdapter` callable.
- Add `GatewayMediator` as the coordination boundary.
- Add reply formatting strategies.
- Add config-driven gateway builder/factory for future CLI migration.
- Migrate CLI gateway startup to the builder so production code no longer calls deprecated lifecycle APIs directly.
- Split Telegram implementation so no source file exceeds 500 lines.
- Mark legacy interfaces deprecated after replacement primitives and compatibility tests exist, but do not delete them.

## Impact

- Affected specs: `gateway-design-patterns`
- Affected code: `macaca-gateway`, `macaca-cli`, gateway integration tests
- Compatibility: existing `Gateway`, `ImAdapter`, `EventHandler`, `TelegramAdapter`, and `DiscordAdapter` remain callable for migration discovery.

## Non-Goals

- Do not connect gateway directly to `chat_v2`.
- Do not add new platforms.
- Do not remove `ImAdapter`, `EventHandler`, `Gateway`, `TelegramAdapter`, or `DiscordAdapter`.
- Do not introduce new third-party dependencies.
- Do not change Telegram/Discord runtime behavior.
