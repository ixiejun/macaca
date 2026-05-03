# macaca-gateway Design Pattern Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor `macaca-gateway` incrementally with Adapter, Bridge, Mediator, Strategy, Factory, and Builder patterns while preserving current gateway behavior 1:1.

**Architecture:** Add platform-neutral gateway primitives first, bridge existing adapters into the new transport boundary, then introduce mediator, formatting strategy, and config-driven builder without deleting old APIs. Keep gateway independent from `macaca-web`, `macaca-kernel`, application names, workflow names, and driver names.

**Tech Stack:** Rust, `async-trait`, `macaca-proto`, existing `reqwest` Telegram integration, existing gateway integration tests.

---

## File Map

- Modify: `macaca/crates/macaca-gateway/src/lib.rs` to export new additive modules.
- Modify: `macaca/crates/macaca-gateway/src/adapter.rs` to mark legacy interface boundaries after new equivalents exist.
- Modify: `macaca/crates/macaca-gateway/src/gateway.rs` to keep existing `Gateway` behavior and add mediator/builder integration points.
- Modify: `macaca/crates/macaca-gateway/src/telegram.rs` only as a compatibility re-export wrapper after splitting.
- Create: `macaca/crates/macaca-gateway/src/message.rs` for platform-neutral inbound/outbound/reply models.
- Create: `macaca/crates/macaca-gateway/src/transport.rs` for `GatewayTransport` and legacy bridge.
- Create: `macaca/crates/macaca-gateway/src/mediator.rs` for `GatewayMediator`.
- Create: `macaca/crates/macaca-gateway/src/format.rs` for reply formatting strategies.
- Create: `macaca/crates/macaca-gateway/src/builder.rs` for config-driven gateway construction.
- Create: `macaca/crates/macaca-gateway/src/telegram/mod.rs`, `telegram/parser.rs`, `telegram/format.rs`, `telegram/client.rs` if splitting requires a directory module.
- Modify: `macaca/crates/macaca-cli/src/commands.rs` to use `GatewayBuilder` instead of direct deprecated lifecycle calls.
- Modify: `macaca/crates/macaca-integration-tests/tests/gateway_pipeline.rs` only to add additive coverage and preserve legacy compatibility tests.

## Task 1: OpenSpec Proposal and Contract

**Files:**
- Create: `openspec/changes/refactor-macaca-gateway-design-patterns/proposal.md`
- Create: `openspec/changes/refactor-macaca-gateway-design-patterns/design.md`
- Create: `openspec/changes/refactor-macaca-gateway-design-patterns/tasks.md`
- Create: `openspec/changes/refactor-macaca-gateway-design-patterns/specs/gateway-design-patterns/spec.md`

- [ ] **Step 1: Create proposal**

Write `proposal.md` with this content:

```markdown
# Refactor macaca-gateway with Design Pattern Primitives

## Why

`macaca-gateway` is the external protocol entry layer for Agent OS. It currently mixes platform adapter lifecycle, platform parsing, reply formatting, and gateway coordination in a small set of concrete types. `telegram.rs` also exceeds the project file-size limit and combines polling, parsing, sending, splitting, and tests.

This change introduces additive-first gateway primitives so future Telegram, Discord, Slack, email, or custom gateway surfaces can be added without hardcoding platform logic into CLI/web/kernel code.

## What Changes

- Add platform-neutral inbound/outbound gateway message primitives.
- Add `GatewayTransport` as the transport boundary while keeping legacy `ImAdapter` callable.
- Add `GatewayMediator` as the coordination boundary.
- Add reply formatting strategies.
- Add config-driven gateway builder/factory for future CLI migration.
- Migrate CLI gateway startup to the builder so production code no longer calls deprecated lifecycle APIs directly.
- Split Telegram implementation so no file exceeds 500 lines.

## Non-Goals

- Do not connect gateway directly to `chat_v2`.
- Do not add new platforms.
- Do not remove `ImAdapter`, `EventHandler`, `Gateway`, `TelegramAdapter`, or `DiscordAdapter`.
- Do not introduce new third-party dependencies.
- Do not change Telegram/Discord runtime behavior.
```

- [ ] **Step 2: Create design**

Write `design.md` with these sections:

```markdown
# Design

## Pattern Mapping

- Adapter: platform raw messages convert into gateway-neutral primitives.
- Bridge: `GatewayTransport` separates platform transport from gateway orchestration.
- Mediator: `GatewayMediator` coordinates message handling and replies.
- Strategy: `GatewayReplyFormatter` handles platform-specific reply formatting.
- Factory/Builder: `GatewayBuilder` constructs gateway instances from config.

## Compatibility

All existing public types remain callable. New APIs are additive. Legacy APIs may be marked deprecated only after new APIs and compatibility tests are in place.

## Dependency Boundary

`macaca-gateway` must remain independent from `macaca-web`, `macaca-kernel`, `macaca-app`, and application-specific concepts. Integration with sessions or chat dispatch must happen through traits in a later consumer migration.

## File Size

`telegram.rs` must be split so each gateway source file stays under 500 lines.

## Risk Controls

Each slice must keep `cargo test -p macaca-gateway -- --nocapture` green. Existing integration tests must continue passing.
```

- [ ] **Step 3: Create tasks**

Write `tasks.md` mirroring Tasks 1-8 in this implementation plan with checkboxes.

- [ ] **Step 4: Create delta spec**

Write `spec.md`:

```markdown
## ADDED Requirements

### Requirement: Platform Neutral Gateway Messages

Gateway MUST expose platform-neutral inbound and outbound message primitives independent of Telegram or Discord wire formats.

#### Scenario: Telegram text maps to neutral inbound message

- **WHEN** a Telegram text message is parsed
- **THEN** the parser produces a platform-neutral inbound message with platform, user id, channel id, and content.

### Requirement: Transport Boundary

Gateway MUST expose a transport trait that separates platform lifecycle and message sending from gateway orchestration.

#### Scenario: Existing adapters remain callable

- **WHEN** existing code constructs `TelegramAdapter` or `DiscordAdapter`
- **THEN** it can still start, send, and stop through the legacy adapter interface.

### Requirement: Gateway Mediator Boundary

Gateway SHOULD provide a mediator boundary for message handling without depending on web, kernel, or application crates.

#### Scenario: Mediator handles task request

- **WHEN** the mediator receives a task request
- **THEN** it dispatches the equivalent existing `GatewayEvent` to an `EventHandler`.

### Requirement: Reply Formatting Strategy

Gateway SHOULD format outgoing replies through platform-specific strategy objects.

#### Scenario: Telegram reply splitting remains stable

- **WHEN** a Telegram reply exceeds Telegram's max message length
- **THEN** it is split with the same newline-preferred behavior as the current implementation.

### Requirement: Config Driven Gateway Builder

Gateway SHOULD provide a builder or factory that constructs configured gateway adapters without caller-side platform branching.

#### Scenario: Disabled adapters are not registered

- **WHEN** gateway config disables Telegram or Discord
- **THEN** the builder does not register that adapter.
```

- [ ] **Step 5: Validate OpenSpec**

Run:

```bash
openspec validate refactor-macaca-gateway-design-patterns --strict
```

Expected: validation passes.

## Task 2: Baseline and Impact Analysis

**Files:**
- Read-only: gateway and consumer files.

- [ ] **Step 1: Run GitNexus impact for gateway symbols**

Run impact analysis for symbols before editing:

```text
Gateway
ImAdapter
EventHandler
TelegramAdapter
DiscordAdapter
```

Expected: report direct callers and warn if any result is HIGH or CRITICAL.

- [ ] **Step 2: Run baseline gateway tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-gateway -- --nocapture
```

Expected: 30 tests pass.

- [ ] **Step 3: Run baseline gateway integration tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-integration-tests gateway -- --nocapture
```

Expected: gateway-related tests pass.

- [ ] **Step 4: Record file sizes**

Run:

```bash
wc -l crates/macaca-gateway/src/*.rs
```

Expected: `telegram.rs` is over 500 lines before refactor and must be split by Task 7.

## Task 3: Platform-Neutral Message Primitives

**Files:**
- Create: `macaca/crates/macaca-gateway/src/message.rs`
- Modify: `macaca/crates/macaca-gateway/src/lib.rs`
- Test: module tests in `message.rs`

- [ ] **Step 1: Add failing tests**

Create `message.rs` tests for:

```rust
#[test]
fn inbound_task_request_converts_to_gateway_event() {
    let message = GatewayInboundMessage::text("telegram", "u1", "c1", "build app");
    let event = message.to_task_request_event();

    match event {
        macaca_proto::GatewayEvent::TaskRequest { user_id, channel_id, content } => {
            assert_eq!(user_id, "u1");
            assert_eq!(channel_id, "c1");
            assert_eq!(content, "build app");
        }
        other => panic!("expected TaskRequest, got {other:?}"),
    }
}

#[test]
fn outbound_reply_carries_target_and_content() {
    let reply = GatewayReply::text("telegram", "c1", "done");
    assert_eq!(reply.platform, "telegram");
    assert_eq!(reply.channel_id, "c1");
    assert_eq!(reply.content, "done");
}
```

- [ ] **Step 2: Run tests and confirm failure**

Run:

```bash
cargo test -p macaca-gateway message -- --nocapture
```

Expected: fails because `message` module/types do not exist.

- [ ] **Step 3: Implement minimal message primitives**

Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayInboundMessage {
    pub platform: String,
    pub user_id: String,
    pub channel_id: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayReply {
    pub platform: String,
    pub channel_id: String,
    pub content: String,
}
```

Add constructors and conversion to `macaca_proto::GatewayEvent::TaskRequest`.

- [ ] **Step 4: Export module**

Update `lib.rs`:

```rust
pub mod message;

pub use message::{GatewayInboundMessage, GatewayReply};
```

- [ ] **Step 5: Verify**

Run:

```bash
cargo test -p macaca-gateway message -- --nocapture
```

Expected: tests pass.

## Task 4: GatewayTransport Boundary

**Files:**
- Create: `macaca/crates/macaca-gateway/src/transport.rs`
- Modify: `macaca/crates/macaca-gateway/src/lib.rs`
- Modify: `macaca/crates/macaca-gateway/src/telegram.rs`
- Modify: `macaca/crates/macaca-gateway/src/discord.rs`

- [ ] **Step 1: Add failing transport tests**

Test that a mock transport can send a `GatewayReply` and that Telegram/Discord expose transport names.

- [ ] **Step 2: Implement `GatewayTransport`**

Add:

```rust
#[async_trait::async_trait]
pub trait GatewayTransport: Send + Sync {
    fn name(&self) -> &str;
    async fn send_reply(&self, reply: &crate::message::GatewayReply) -> macaca_proto::MacacaResult<()>;
    async fn stop(&self) -> macaca_proto::MacacaResult<()>;
}
```

- [ ] **Step 3: Implement for existing adapters**

Implement `GatewayTransport` for `TelegramAdapter` and `DiscordAdapter` by forwarding `send_reply` to existing `send_message`.

- [ ] **Step 4: Keep legacy interfaces**

Do not remove `ImAdapter`. Do not change `Gateway::register_adapter`.

- [ ] **Step 5: Verify**

Run:

```bash
cargo test -p macaca-gateway transport -- --nocapture
cargo test -p macaca-gateway -- --nocapture
```

Expected: all pass.

## Task 5: GatewayMediator Boundary

**Files:**
- Create: `macaca/crates/macaca-gateway/src/mediator.rs`
- Modify: `macaca/crates/macaca-gateway/src/lib.rs`

- [ ] **Step 1: Add mediator tests**

Test that mediator receives `GatewayInboundMessage` and dispatches equivalent `GatewayEvent::TaskRequest` to an `EventHandler`.

- [ ] **Step 2: Implement mediator**

Add:

```rust
pub struct GatewayMediator {
    handler: std::sync::Arc<dyn crate::EventHandler>,
}
```

Add `handle_inbound(&self, message: GatewayInboundMessage) -> MacacaResult<Option<GatewayReply>>`.

- [ ] **Step 3: Keep scope narrow**

Do not reference web, kernel, chat_v2, application ids, app names, workflow names, or driver names.

- [ ] **Step 4: Verify**

Run:

```bash
cargo test -p macaca-gateway mediator -- --nocapture
```

Expected: mediator tests pass.

## Task 6: Reply Formatting Strategy

**Files:**
- Create: `macaca/crates/macaca-gateway/src/format.rs`
- Modify: `macaca/crates/macaca-gateway/src/lib.rs`
- Eventually move: `split_message` behavior from Telegram module behind strategy.

- [ ] **Step 1: Add formatter tests**

Cover:

- Plain formatter returns one chunk for short text.
- Telegram formatter splits long text at newline when possible.
- Telegram formatter hard-splits when no newline exists.

- [ ] **Step 2: Implement strategy trait**

Add:

```rust
pub trait GatewayReplyFormatter: Send + Sync {
    fn format(&self, reply: &crate::GatewayReply) -> Vec<String>;
}
```

Add `PlainTextFormatter` and `TelegramFormatter`.

- [ ] **Step 3: Preserve split behavior**

Use the current `split_message` algorithm exactly. Keep existing Telegram split tests passing.

- [ ] **Step 4: Verify**

Run:

```bash
cargo test -p macaca-gateway format -- --nocapture
cargo test -p macaca-gateway telegram::tests::test_split_message -- --nocapture
```

Expected: formatter and existing split tests pass.

## Task 7: Split Telegram Module Under 500 Lines

**Files:**
- Modify/Create: `macaca/crates/macaca-gateway/src/telegram/mod.rs`
- Create: `macaca/crates/macaca-gateway/src/telegram/parser.rs`
- Create: `macaca/crates/macaca-gateway/src/telegram/format.rs`
- Create: `macaca/crates/macaca-gateway/src/telegram/client.rs`
- Delete or shrink: `macaca/crates/macaca-gateway/src/telegram.rs`

- [ ] **Step 1: Move parser tests with parser**

Move `parse_message` and parse tests into `telegram/parser.rs`.

- [ ] **Step 2: Move split tests with formatting**

Move `split_message` and split tests into `telegram/format.rs` or use `format.rs` strategy tests.

- [ ] **Step 3: Keep public compatibility**

Ensure `TelegramAdapter` remains importable as:

```rust
use macaca_gateway::TelegramAdapter;
```

- [ ] **Step 4: Verify file size**

Run:

```bash
wc -l crates/macaca-gateway/src/telegram*.rs crates/macaca-gateway/src/telegram/*.rs
```

Expected: every file is below 500 lines.

- [ ] **Step 5: Verify tests**

Run:

```bash
cargo test -p macaca-gateway telegram -- --nocapture
```

Expected: all Telegram tests pass.

## Task 8: Config-Driven Gateway Builder

**Files:**
- Create: `macaca/crates/macaca-gateway/src/builder.rs`
- Modify: `macaca/crates/macaca-gateway/src/lib.rs`

- [ ] **Step 1: Add builder tests**

Cover:

- disabled gateway registers no adapters.
- enabled Telegram registers one adapter.
- enabled Telegram + Discord registers two adapters.

- [ ] **Step 2: Implement builder**

Add:

```rust
pub struct GatewayBuilder {
    config: macaca_proto::config::GatewayConfig,
    handler: std::sync::Arc<dyn crate::EventHandler>,
}
```

Add `build(self) -> Gateway` preserving existing registration behavior.

- [ ] **Step 3: Migrate CLI startup to builder**

Update `macaca-cli/src/commands.rs` so gateway startup uses:

```rust
let gateway = GatewayBuilder::new(config.gateway.clone()).start().await?;
```

Expected: CLI production code no longer directly calls `Gateway::new`, `register_adapter`, or `start_all`.

- [ ] **Step 4: Verify**

Run:

```bash
cargo test -p macaca-gateway builder -- --nocapture
```

Expected: builder tests pass.

## Task 9: Final Verification

**Files:**
- All gateway files touched above.

- [ ] **Step 1: Format**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo fmt
```

Expected: no formatting diff remains.

- [ ] **Step 2: Gateway tests**

Run:

```bash
cargo test -p macaca-gateway -- --nocapture
```

Expected: all pass.

- [ ] **Step 3: Gateway integration tests**

Run:

```bash
cargo test -p macaca-integration-tests gateway -- --nocapture
```

Expected: gateway-related tests pass.

- [ ] **Step 4: Check gateway and consumers**

Run:

```bash
cargo check -p macaca-gateway -p macaca-cli -p macaca-integration-tests
```

Expected: passes with only existing warnings.

- [ ] **Step 5: OpenSpec validation**

Run:

```bash
openspec validate refactor-macaca-gateway-design-patterns --strict
```

Expected: passes.

- [ ] **Step 6: GitNexus detect changes**

Run GitNexus detect changes for all uncommitted changes.

Expected: affected scope matches gateway and planned tests/docs only.

## Self-Review

- Spec coverage: all five intended slices are represented: neutral messages, transport, mediator, formatting strategy, builder/factory, plus Telegram file split.
- Placeholder scan: no unresolved placeholders are intentionally left.
- Type consistency: plan uses `GatewayInboundMessage`, `GatewayReply`, `GatewayTransport`, `GatewayMediator`, and `GatewayReplyFormatter` consistently.
- Scope check: CLI production startup migration is included because legacy lifecycle APIs are deprecated in this proposal; web/session/chat dispatch integration remains excluded.
