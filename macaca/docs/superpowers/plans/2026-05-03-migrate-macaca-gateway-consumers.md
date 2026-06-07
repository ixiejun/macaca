# Migrate macaca-gateway Consumers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 `macaca-gateway` 的上层消费方迁移到本次设计模式重构后的 builder / mediator primitives，生产路径不再直接调用 deprecated gateway lifecycle API，并保持旧 API 兼容测试。

**Architecture:** 采用 additive-first 消费方迁移。`macaca-cli` 作为唯一生产消费方使用 `GatewayBuilder`；`macaca-integration-tests` 新增 builder / mediator 覆盖，同时保留少量 legacy compatibility tests；`macaca-gateway` 内部 compatibility bridge 不在本轮强行替换。

**Tech Stack:** Rust, Tokio, `macaca-gateway`, `macaca-cli`, `macaca-integration-tests`, OpenSpec, GitNexus, cargo test/check.

---

## Current Context

当前 `macaca-gateway` 重构已引入：

- `macaca/crates/macaca-gateway/src/builder.rs`：`GatewayBuilder`。
- `macaca/crates/macaca-gateway/src/mediator.rs`：`GatewayMediator` 和 `GatewayEventSink`。
- `macaca/crates/macaca-gateway/src/transport.rs`：`GatewayTransport`。
- `macaca/crates/macaca-gateway/src/message.rs`：`GatewayInboundMessage` / `GatewayReply`。
- `macaca/crates/macaca-gateway/src/format.rs`：`GatewayReplyFormatter`。

上层消费扫描结果：

- `macaca-cli` 是唯一生产消费方，当前已经导入并使用 `GatewayBuilder`。
- `macaca-integration-tests` 是唯一测试消费方，当前仍直接使用 deprecated `Gateway` / `ImAdapter` / `EventHandler` 路径。
- `macaca-web` 没有直接依赖 `macaca-gateway`，本轮不应新增依赖。

当前允许保留 deprecated 调用的位置：

- `macaca-gateway` crate 内部 compatibility bridge 和单测。
- `macaca-integration-tests` 中明确命名的 legacy compatibility tests。

当前不允许保留 deprecated 调用的位置：

- `macaca-cli` 生产路径。
- 未来任何 `macaca-web` / `macaca-app` / `macaca-kernel` 生产路径。

## Superpowers Brainstorm Summary

推荐方案是“生产路径迁移 + integration tests 双轨覆盖”：

- 保持 CLI 使用 `GatewayBuilder`。
- 为 integration tests 新增 `GatewayBuilder` 和 `GatewayMediator` 消费路径测试。
- 保留旧 API compatibility tests，避免 deprecated API 虽然保留但失去跨 crate 兼容保护。
- 用 grep 明确生产上层禁止 deprecated gateway API，而不是全仓库禁止。

不采用的方案：

- 只确认 CLI 已迁移：测试层缺少新入口覆盖。
- 删除全部 legacy integration tests：降低兼容性保护，和 deprecated-but-callable 策略冲突。

## File Map

- Create: `openspec/changes/migrate-gateway-consumers-to-pattern-primitives/proposal.md`
- Create: `openspec/changes/migrate-gateway-consumers-to-pattern-primitives/design.md`
- Create: `openspec/changes/migrate-gateway-consumers-to-pattern-primitives/tasks.md`
- Create: `openspec/changes/migrate-gateway-consumers-to-pattern-primitives/specs/macaca-gateway-consumers/spec.md`
- Modify: `macaca/crates/macaca-cli/src/commands.rs`
- Modify: `macaca/crates/macaca-integration-tests/tests/gateway_pipeline.rs`

## Task 1: Create OpenSpec Change

**Files:**

- Create: `openspec/changes/migrate-gateway-consumers-to-pattern-primitives/proposal.md`
- Create: `openspec/changes/migrate-gateway-consumers-to-pattern-primitives/design.md`
- Create: `openspec/changes/migrate-gateway-consumers-to-pattern-primitives/tasks.md`
- Create: `openspec/changes/migrate-gateway-consumers-to-pattern-primitives/specs/macaca-gateway-consumers/spec.md`

- [ ] **Step 1: Review OpenSpec context**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec list
openspec list --specs
```

Expected:

```text
refactor-macaca-gateway-design-patterns is present and complete
migrate-gateway-consumers-to-pattern-primitives does not already exist
```

- [ ] **Step 2: Create proposal**

Create `openspec/changes/migrate-gateway-consumers-to-pattern-primitives/proposal.md`:

```markdown
# Change: Migrate macaca-gateway consumers to pattern primitives

## Why

`macaca-gateway` now exposes builder, mediator, transport, message, and formatter primitives. Upper production code should consume those primitives instead of directly constructing legacy gateway lifecycle objects, while compatibility tests should continue protecting the deprecated API until it can be removed in a later migration.

## What Changes

- Require production gateway consumers to use `GatewayBuilder` or future non-deprecated gateway primitives.
- Keep `macaca-cli` gateway startup on `GatewayBuilder`.
- Add integration coverage for `GatewayBuilder` and `GatewayMediator` consumer paths.
- Restrict direct legacy `Gateway` / `ImAdapter` / `EventHandler` usage to gateway internals and explicitly named compatibility tests.

## Impact

- Affected specs: `macaca-gateway-consumers`
- Affected code: `macaca-cli`, `macaca-integration-tests`
- Non-impact: no Telegram/Discord runtime behavior change; no web/session/chat_v2 integration.
```

- [ ] **Step 3: Create design**

Create `openspec/changes/migrate-gateway-consumers-to-pattern-primitives/design.md`:

```markdown
## Context

The gateway refactor introduced additive primitives while keeping legacy interfaces deprecated but callable. Direct Cargo consumers of `macaca-gateway` are currently limited to `macaca-cli` and `macaca-integration-tests`.

## Goals

- Ensure production upper crates do not call deprecated gateway lifecycle APIs directly.
- Add cross-crate tests for the new builder and mediator primitives.
- Preserve legacy compatibility tests for deprecated APIs while they remain public.
- Keep gateway independent from web, kernel, app, workflow, driver, and application-specific names.

## Non-Goals

- Do not remove deprecated gateway APIs.
- Do not replace gateway internal compatibility bridge in this change.
- Do not add new gateway platforms.
- Do not connect gateway to chat_v2, sessions, EventLog, or application runtimes.

## Decisions

- `macaca-cli` uses `GatewayBuilder` for production startup.
- `macaca-integration-tests` contains both new primitive tests and explicitly named legacy compatibility tests.
- Deprecated usage verification distinguishes production upper crates from compatibility tests.
```

- [ ] **Step 4: Create tasks**

Create `openspec/changes/migrate-gateway-consumers-to-pattern-primitives/tasks.md`:

```markdown
## 1. Preparation

- [ ] 1.1 Run GitNexus impact for `run_kernel` before editing CLI startup.
- [ ] 1.2 Run deprecated usage grep across direct gateway consumers.
- [ ] 1.3 Confirm direct Cargo consumers are only `macaca-cli` and `macaca-integration-tests`.

## 2. Production consumer migration

- [ ] 2.1 Keep or migrate `macaca-cli::run_kernel` to `GatewayBuilder`.
- [ ] 2.2 Verify `macaca-cli` does not import `Gateway`, `ImAdapter`, `EventHandler`, `TelegramAdapter`, or `DiscordAdapter`.

## 3. Integration coverage migration

- [ ] 3.1 Add or keep a builder lifecycle integration test.
- [ ] 3.2 Add a mediator/event sink integration test.
- [ ] 3.3 Rename legacy lifecycle tests to make compatibility intent explicit.
- [ ] 3.4 Keep deprecated allowance scoped to compatibility tests only where practical.

## 4. Verification

- [ ] 4.1 Run `cargo fmt`.
- [ ] 4.2 Run `cargo test -p macaca-gateway -- --nocapture`.
- [ ] 4.3 Run `cargo test -p macaca-integration-tests gateway -- --nocapture`.
- [ ] 4.4 Run `cargo check -p macaca-gateway -p macaca-cli -p macaca-integration-tests`.
- [ ] 4.5 Run production deprecated usage grep.
- [ ] 4.6 Run `openspec validate migrate-gateway-consumers-to-pattern-primitives --strict`.
- [ ] 4.7 Run `gitnexus_detect_changes(scope: "all")` before commit.
```

- [ ] **Step 5: Create delta spec**

Create `openspec/changes/migrate-gateway-consumers-to-pattern-primitives/specs/macaca-gateway-consumers/spec.md`:

```markdown
## ADDED Requirements

### Requirement: Production Gateway Consumers Use Pattern Primitives

Production upper crates that start gateway adapters SHALL use non-deprecated gateway primitives such as `GatewayBuilder` instead of directly calling deprecated gateway lifecycle APIs.

#### Scenario: CLI starts gateway from configuration

- **WHEN** gateway is enabled in CLI configuration
- **THEN** CLI starts it through `GatewayBuilder`
- **AND** CLI does not manually register Telegram or Discord adapters.

### Requirement: Gateway Consumer Tests Cover New Primitives

Gateway integration tests SHALL cover the new builder and mediator primitives.

#### Scenario: Builder constructs enabled adapters

- **WHEN** integration tests build a gateway from enabled Telegram and Discord config
- **THEN** the resulting gateway contains both adapters.

#### Scenario: Mediator dispatches inbound messages

- **WHEN** integration tests send a platform-neutral inbound message to `GatewayMediator`
- **THEN** the configured event sink receives the equivalent gateway event.

### Requirement: Legacy Gateway Compatibility Remains Explicit

Deprecated gateway APIs MAY remain used only in gateway internals or explicitly named compatibility tests.

#### Scenario: Deprecated lifecycle API remains callable

- **WHEN** a compatibility test constructs a legacy `Gateway`
- **THEN** it can register, start, and stop adapters without changing existing behavior.
```

- [ ] **Step 6: Validate OpenSpec**

Run:

```bash
openspec validate migrate-gateway-consumers-to-pattern-primitives --strict
```

Expected:

```text
Change 'migrate-gateway-consumers-to-pattern-primitives' is valid
```

## Task 2: Read and Impact Analysis

**Files:**

- Read: `macaca/crates/macaca-cli/src/commands.rs`
- Read: `macaca/crates/macaca-integration-tests/tests/gateway_pipeline.rs`
- Read: `macaca/crates/macaca-gateway/src/builder.rs`
- Read: `macaca/crates/macaca-gateway/src/mediator.rs`

- [ ] **Step 1: Confirm direct consumers**

Run:

```bash
cd /Users/quantum/Code/dev/agent
rg -n "macaca-gateway|macaca_gateway" macaca/Cargo.toml macaca/crates/*/Cargo.toml
```

Expected direct consumers:

```text
macaca/crates/macaca-cli/Cargo.toml
macaca/crates/macaca-integration-tests/Cargo.toml
```

- [ ] **Step 2: Confirm deprecated usage shape**

Run:

```bash
rg -n "Gateway::new|register_adapter|start_all|stop_all|ImAdapter|EventHandler|TelegramAdapter|DiscordAdapter" macaca/crates/macaca-cli/src macaca/crates/macaca-integration-tests/tests macaca/crates/macaca-gateway/src --glob '!target'
```

Expected:

- `macaca-cli` should not match deprecated gateway lifecycle APIs.
- `macaca-integration-tests` may match compatibility tests before Task 4.
- `macaca-gateway` may match internal bridge/tests.

- [ ] **Step 3: Run GitNexus impact before any symbol edit**

Run impact only if implementation will edit the symbol:

```text
impact target: run_kernel, direction: upstream
impact target: register_and_start_stop_adapters, direction: upstream
impact target: gateway_with_counting_handler_lifecycle, direction: upstream
```

Expected:

- `run_kernel` may show HIGH/CRITICAL because it is called by CLI `main`; if so, report the risk and keep the edit limited to gateway startup.
- Integration test symbols should be low risk.

## Task 3: Ensure CLI Production Path Uses GatewayBuilder

**Files:**

- Modify if needed: `macaca/crates/macaca-cli/src/commands.rs`

- [ ] **Step 1: Inspect current import**

Verify the file imports `GatewayBuilder`:

```rust
use macaca_gateway::GatewayBuilder;
```

- [ ] **Step 2: Inspect startup branch**

Verify the gateway-enabled branch is:

```rust
if config.gateway.enabled {
    let gateway = GatewayBuilder::new(config.gateway.clone()).start().await?;
    info!(
        adapters = gateway.adapter_count(),
        "Gateway adapters running"
    );
}
```

- [ ] **Step 3: If direct legacy construction exists, replace it**

Replace any code shaped like this:

```rust
let mut gateway = Gateway::new(handler);
gateway.register_adapter(Box::new(TelegramAdapter::new(config)));
gateway.start_all().await?;
```

with:

```rust
let gateway = GatewayBuilder::new(config.gateway.clone()).start().await?;
info!(
    adapters = gateway.adapter_count(),
    "Gateway adapters running"
);
```

- [ ] **Step 4: Verify CLI deprecated usage is gone**

Run:

```bash
rg -n "Gateway::new|register_adapter|start_all|stop_all|ImAdapter|EventHandler|TelegramAdapter|DiscordAdapter" macaca/crates/macaca-cli/src
```

Expected:

```text
no matches
```

## Task 4: Add New Primitive Integration Coverage

**Files:**

- Modify: `macaca/crates/macaca-integration-tests/tests/gateway_pipeline.rs`

- [ ] **Step 1: Add imports for new primitives**

Add these imports beside existing compatibility imports:

```rust
use macaca_gateway::{
    GatewayBuilder, GatewayEventSink, GatewayInboundMessage, GatewayMediator,
};
```

If this conflicts with grouped imports, keep one grouped `use macaca_gateway::{ ... };` block.

- [ ] **Step 2: Add builder lifecycle test**

Add this test:

```rust
#[tokio::test]
async fn builder_constructs_configured_gateway_adapters() {
    let gateway = GatewayBuilder::new(macaca_proto::config::GatewayConfig {
        enabled: true,
        telegram: Some(telegram_config()),
        discord: Some(discord_config()),
    })
    .start()
    .await
    .unwrap();

    assert_eq!(gateway.adapter_count(), 2);
    gateway.stop_all().await.unwrap();
}
```

Note: `stop_all()` is still legacy on the returned compatibility `Gateway`. If warning scope becomes noisy, keep this test under the compatibility allowance until the builder returns a non-deprecated runtime handle in a later gateway-internal migration.

- [ ] **Step 3: Add mediator sink**

Add a test sink near `CountingEventHandler`:

```rust
struct CountingEventSink {
    count: AtomicUsize,
}

impl CountingEventSink {
    fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
        }
    }

    fn event_count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl GatewayEventSink for CountingEventSink {
    async fn dispatch(&self, event: GatewayEvent) -> MacacaResult<()> {
        match event {
            GatewayEvent::TaskRequest { content, .. } => {
                assert_eq!(content, "write a status report");
            }
            other => panic!("expected TaskRequest, got {other:?}"),
        }
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
```

- [ ] **Step 4: Add mediator integration test**

Add this test:

```rust
#[tokio::test]
async fn mediator_dispatches_platform_neutral_inbound_message() {
    let sink = Arc::new(CountingEventSink::new());
    let mediator = GatewayMediator::new(sink.clone());

    mediator
        .handle_inbound(GatewayInboundMessage::text(
            "telegram",
            "user1",
            "chat1",
            "write a status report",
        ))
        .await
        .unwrap();

    assert_eq!(sink.event_count(), 1);
}
```

- [ ] **Step 5: Rename old lifecycle tests for compatibility intent**

Rename tests without changing behavior:

```rust
register_and_start_stop_adapters
```

to:

```rust
legacy_gateway_register_and_start_stop_adapters_compatibility
```

Rename:

```rust
gateway_with_counting_handler_lifecycle
```

to:

```rust
legacy_gateway_with_counting_handler_lifecycle_compatibility
```

Rename:

```rust
empty_gateway_start_stop
```

to:

```rust
legacy_empty_gateway_start_stop_compatibility
```

## Task 5: Verification

**Files:**

- No source edits unless checks fail.

- [ ] **Step 1: Format**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo fmt
```

Expected: command exits 0.

- [ ] **Step 2: Gateway unit tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-gateway -- --nocapture
```

Expected: all tests pass.

- [ ] **Step 3: Gateway integration tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-integration-tests gateway -- --nocapture
```

Expected: all gateway integration tests pass, including new builder and mediator tests.

- [ ] **Step 4: Targeted check**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo check -p macaca-gateway -p macaca-cli -p macaca-integration-tests
```

Expected: command exits 0. Existing unrelated warnings may remain.

- [ ] **Step 5: Production deprecated usage grep**

Run:

```bash
cd /Users/quantum/Code/dev/agent
rg -n "Gateway::new|register_adapter|start_all|stop_all|ImAdapter|EventHandler|TelegramAdapter|DiscordAdapter" macaca/crates/macaca-cli/src
```

Expected:

```text
no matches
```

- [ ] **Step 6: Full deprecated usage audit**

Run:

```bash
cd /Users/quantum/Code/dev/agent
rg -n "Gateway::new|register_adapter|start_all|stop_all|ImAdapter|EventHandler|TelegramAdapter|DiscordAdapter" macaca/crates --glob '!target'
```

Expected remaining matches only in:

- `macaca/crates/macaca-gateway/src/*` internal compatibility bridge/tests.
- `macaca/crates/macaca-integration-tests/tests/gateway_pipeline.rs` explicitly named legacy compatibility tests.

- [ ] **Step 7: OpenSpec validation**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec validate migrate-gateway-consumers-to-pattern-primitives --strict
```

Expected:

```text
Change 'migrate-gateway-consumers-to-pattern-primitives' is valid
```

- [ ] **Step 8: GitNexus change detection**

Run:

```text
gitnexus_detect_changes(scope: "all")
```

Expected:

- Changed symbols are limited to OpenSpec/docs, CLI gateway startup if it needed edits, and gateway integration tests.
- Any affected process involving CLI `main` is expected only if `run_kernel` changed.

## Self-Review

- Spec coverage: The plan covers production consumer migration, new primitive integration coverage, explicit legacy compatibility, and verification.
- Placeholder scan: No placeholder markers remain.
- Type consistency: The plan uses existing public types from the current gateway refactor: `GatewayBuilder`, `GatewayMediator`, `GatewayEventSink`, and `GatewayInboundMessage`.
- Scope control: The plan intentionally does not modify gateway internal lifecycle bridge or connect gateway to web/session/chat runtime.
