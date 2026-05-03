## 1. OpenSpec

- [x] 1.1 Create proposal, design, tasks, and delta spec.
- [x] 1.2 Validate `refactor-macaca-gateway-design-patterns` with `openspec validate --strict`.

## 2. Baseline and Impact

- [x] 2.1 Run GitNexus impact for `Gateway`.
- [x] 2.2 Run GitNexus impact for `ImAdapter`.
- [x] 2.3 Run GitNexus impact for `EventHandler`.
- [x] 2.4 Run GitNexus impact for `TelegramAdapter`.
- [x] 2.5 Run GitNexus impact for `DiscordAdapter`.
- [x] 2.6 Run baseline `cargo test -p macaca-gateway -- --nocapture`.
- [x] 2.7 Run baseline `cargo test -p macaca-integration-tests gateway -- --nocapture`.
- [x] 2.8 Record gateway file sizes and confirm `telegram.rs` exceeds 500 lines before splitting.

## 3. Platform-Neutral Message Primitives

- [x] 3.1 Add `message.rs` with `GatewayInboundMessage`, `GatewayOutboundMessage`, and `GatewayReply`.
- [x] 3.2 Add conversion from inbound messages to existing `GatewayEvent`.
- [x] 3.3 Export message primitives from `lib.rs`.
- [x] 3.4 Add message unit tests.

## 4. GatewayTransport Boundary

- [x] 4.1 Add `transport.rs` with `GatewayTransport`.
- [x] 4.2 Implement `GatewayTransport` for `TelegramAdapter`.
- [x] 4.3 Implement `GatewayTransport` for `DiscordAdapter`.
- [x] 4.4 Add transport unit tests.

## 5. GatewayMediator Boundary

- [x] 5.1 Add `mediator.rs` with `GatewayMediator`.
- [x] 5.2 Dispatch neutral inbound messages through existing event sink.
- [x] 5.3 Add mediator unit tests.

## 6. Reply Formatting Strategy

- [x] 6.1 Add `format.rs` with `GatewayReplyFormatter`.
- [x] 6.2 Add `PlainTextFormatter`.
- [x] 6.3 Add `TelegramFormatter` preserving current split behavior.
- [x] 6.4 Add formatter unit tests.

## 7. Telegram Module Split

- [x] 7.1 Split Telegram parser into a dedicated parser module.
- [x] 7.2 Split Telegram formatting into a dedicated formatting module.
- [x] 7.3 Keep `TelegramAdapter` public import compatibility.
- [x] 7.4 Verify all gateway source files are under 500 lines.

## 8. Config-Driven Builder

- [x] 8.1 Add `builder.rs` with `GatewayBuilder`.
- [x] 8.2 Build configured adapters from `GatewayConfig`.
- [x] 8.3 Add builder tests for disabled and enabled adapter combinations.

## 9. Deprecation

- [x] 9.1 Mark legacy `ImAdapter` as deprecated but keep it callable.
- [x] 9.2 Mark legacy `EventHandler` as deprecated but keep it callable.
- [x] 9.3 Mark legacy `Gateway` constructor/registration lifecycle API as deprecated but keep it callable.
- [x] 9.4 Use local `#[allow(deprecated)]` only in compatibility bridges/tests/current legacy consumers.
- [x] 9.5 Migrate CLI gateway startup to `GatewayBuilder` so production CLI code does not call deprecated lifecycle APIs directly.

## 10. Verification

- [x] 10.1 Run `cargo fmt`.
- [x] 10.2 Run `cargo test -p macaca-gateway -- --nocapture`.
- [x] 10.3 Run `cargo test -p macaca-integration-tests gateway -- --nocapture`.
- [x] 10.4 Run `cargo check -p macaca-gateway -p macaca-cli -p macaca-integration-tests`.
- [x] 10.5 Run `openspec validate refactor-macaca-gateway-design-patterns --strict`.
- [x] 10.6 Run deprecated usage grep for legacy gateway APIs.
- [x] 10.7 Run `gitnexus_detect_changes(scope: "all")`.
