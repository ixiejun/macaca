# macaca-driver 渐进式重构实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 对 `macaca-driver` 做小步、可回滚、行为 1:1 还原的设计模式渐进式重构，收敛 driver 创建、动态 ABI、命令执行、trace 转换和 session 状态边界。

**Architecture:** 本计划采用 additive-first 策略：先新增 factory / command / adapter / proxy / state 原语，再逐步让现有 `DriverLoader`、`DynamicDriver`、`DriverRegistry`、`DynamicTool` 调用新入口。旧接口暂不删除，只标记 deprecated 或保留兼容壳，便于上层消费方分批迁移。

**Tech Stack:** Rust, async_trait, tokio, libloading, macaca-tools, macaca-proto, OpenSpec, GitNexus.

---

## 1. 背景与约束

本计划依据：

- `macaca/docs/design-pattern-refactor-plans/README.md`
- `macaca/docs/design-pattern-refactor-plans/refactor-order.md`
- `macaca/docs/design-pattern-refactor-plans/macaca-driver.md`
- `macaca/docs/design_patterns.md`
- 当前 `macaca/crates/macaca-driver` 源码

`macaca-driver` 位于阶段 2：自主运行基础设施层。它依赖 `macaca-proto` 和 `macaca-tools`，被 `macaca-web`、`macaca-framework`、`macaca-runtime`、integration tests 等消费，是外部执行器进入 Agent OS 的扩展点。

强约束：

- 不硬编码 workflow、application name、driver name 或业务语义。
- 不改变现有 driver ABI 行为。
- 不降低 trace/session/resume 可观测性。
- 不一次性删除旧 API。
- 每个切片独立编译、独立测试、独立可回滚。

## 2. 当前实现观察

### 2.1 现有文件职责

- `macaca/crates/macaca-driver/src/driver.rs`
  - 定义 `SoftwareDriver`、`DriverManifest`、`DriverType`。
  - 当前是 driver contract 的核心。

- `macaca/crates/macaca-driver/src/loader.rs`
  - 扫描 driver 目录、解析 `driver.toml`、检查 host ABI、调用 `DynamicDriver::load`。
  - 同时承担 discovery、validation、dynamic factory 三类职责。

- `macaca/crates/macaca-driver/src/dynamic_driver.rs`
  - 加载动态库、解析 symbol、创建 opaque handle、转换 ABI manifest、暴露 dynamic tool proxy、处理 streaming trace。
  - 文件约 500 行，包含 ABI proxy、tool proxy、trace bridge、错误转换等多重职责。

- `macaca/crates/macaca-driver/src/plugin_abi.rs`
  - 定义 C-ABI 函数签名和 JSON exchange type。
  - 当前适合继续作为稳定 contract，不应在第一轮改变。

- `macaca/crates/macaca-driver/src/sdk.rs`
  - `export_driver!` 宏生成动态 driver 的 C-ABI 符号。
  - 包含单实例 storage、runtime bridge、streaming trace forwarding。

- `macaca/crates/macaca-driver/src/registry.rs`
  - 注册 driver，列 manifest，聚合 tools。
  - 当前是轻量 registry，但没有 lifecycle facade 和 query command 边界。

- `macaca/crates/macaca-driver/src/toolset.rs`
  - 已经是 `macaca_tools::CompositeToolSet` 兼容壳，旧 `new/empty` 已 deprecated。

### 2.2 上层消费点

- `macaca/crates/macaca-web/src/lib.rs`
  - 启动时使用 `DriverLoader::load_all()` 自动加载动态 driver。

- `macaca/crates/macaca-web/src/routes.rs`
  - `/api/drivers/reload` 直接使用 `DriverLoader`，清空并重载 registry。

- `macaca/crates/macaca-web/src/framework_toolkit.rs`
  - 运行时从 `DriverRegistry::aggregate_tools()` 聚合 driver tools。

- `macaca/crates/macaca-framework/src/adapter.rs`
  - 通过 `SingleToolAdapter` 把 `macaca_tools::Tool` 接入 framework toolkit。

- `macaca/crates/macaca-runtime/src/agentic_loop.rs`
  - 将 `macaca_tools::TraceEvent` 转成 `AgentExecutionEvent::DriverTrace`。

## 3. Superpowers Brainstorm

### 方案 A：严格按五个切片逐层抽象

先加 `DriverFactory`，再加 `DriverCommand`，再加 `DriverTraceAdapter`，再抽 `DynamicDriverProxy`，最后加 `DriverSessionState`。

优点：

- 与 `macaca-driver.md` 完全一致。
- 每个切片边界清晰，最容易 code review。
- 兼容 additive-first 和行为 1:1 还原。

风险：

- 上层迁移要等多个切片后才能看到明显收益。
- 如果每个切片不补足测试，后续容易只堆类型不收敛消费方。

### 方案 B：先切 `DynamicDriver` 大文件

优先把 `dynamic_driver.rs` 拆成 ABI resolver、manifest adapter、dynamic tool proxy、streaming bridge，再从拆分后的模块上引入 factory/command/state。

优点：

- 直接降低最大风险文件复杂度。
- 对后续 proxy/factory 更自然。

风险：

- 第一轮改动会比较大，容易触碰 ABI、drop order、unsafe 边界。
- 行为 1:1 回归压力高，不符合“每次只完成一小点”的偏好。

### 方案 C：先从 trace/command 用户可见问题切入

优先建立 `DriverCommand` 和 `DriverTraceAdapter`，让 driver action 与 UI trace label 标准化，再回头收敛 factory/proxy/session。

优点：

- 直接服务用户可见 trace 透明度目标。
- 容易和最近修复的 `Macaca Framework` 重复 trace 问题衔接。

风险：

- 如果底层 factory/proxy 不稳，trace adapter 容易变成又一个补丁层。
- 可能绕开 `macaca-driver.md` 计划中的第一切片，导致文档和实现不一致。

### 推荐方案

采用方案 A，但把第 3 切片的 trace adapter 设计前置到第 1 切片的类型草图中，避免后续重新命名或重排。

原因：

- `macaca-driver` 是基础设施 crate，不应为了短期 UI 症状牺牲底层抽象顺序。
- 当前 `DriverToolSet` 已经完成一部分迁移，下一步最自然的是建立 driver 创建入口和命令原语。
- Factory / Command / Adapter / Proxy / State 能分别对应五个可审查切片。

## 4. 风险清单

- **ABI 风险：** `DynamicDriver::load` 涉及 `libloading`、C string ownership、drop order、unsafe function pointer。任何拆分都必须保留 `_library` 最后 drop 的语义。

- **Trace 风险：** driver trace 当前跨 `macaca-tools::TraceEvent`、`macaca-proto::AgentExecutionEvent::DriverTrace`、`macaca-web` SSE/EventLog 和 frontend renderer。重构时不能丢失 `driver_id`、`timestamp`、`tool_name`、`tool_input`、`tool_output`。

- **Registry 风险：** `/api/drivers/reload` 会清空并重载 registry。若 factory 或 lifecycle facade 行为不同，可能导致 agent runtime 看不到新 driver tools。

- **Session 风险：** `execute_streaming` 的 callback forwarding 线程依赖函数返回前 join，不能引入后台泄漏或 callback pointer 悬挂。

- **兼容风险：** `DriverToolSet::new/empty` 已 deprecated，但可能仍被外部代码或未索引代码使用。继续保留兼容壳。

- **测试风险：** 当前 dynamic driver 很难纯单测真实动态库。需要用小型 fake ABI 单元测试、registry/loader fixture 和 integration smoke test 分层覆盖。

## 5. 目标设计

### 5.1 Factory

新增 `factory.rs`，提供统一 driver 创建入口：

```rust
pub struct DriverCreateContext {
    pub config_json: String,
}

pub trait DriverFactory: Send + Sync {
    fn driver_name(&self) -> &str;
    fn create(&self, ctx: DriverCreateContext) -> MacacaResult<Box<dyn SoftwareDriver>>;
}

pub struct DynamicDriverFactory {
    pub name: String,
    pub library_path: std::path::PathBuf,
}
```

第一轮只让 `DriverLoader::load_driver` 使用 `DynamicDriverFactory`，不改变外部 API。

### 5.2 Command

新增 `command.rs`，把 driver action 表达为 command object：

```rust
pub enum DriverCommand {
    Execute {
        driver_name: String,
        tool_name: String,
        input: serde_json::Value,
    },
    HealthCheck {
        driver_name: String,
    },
    Shutdown {
        driver_name: String,
    },
}
```

第一轮不强行改所有执行路径，只先覆盖 dynamic tool 的 execute/execute_streaming 内部描述和测试。

### 5.3 Trace Adapter

新增 `trace.rs`，统一 driver trace 转换：

```rust
pub struct DriverTraceAdapter;

impl DriverTraceAdapter {
    pub fn normalize_driver_id(event: &mut macaca_tools::TraceEvent, fallback: &str) {
        if event.driver_id.is_none() {
            event.driver_id = Some(fallback.to_string());
        }
    }
}
```

先把 `dynamic_driver.rs` 里 streaming trampoline 的 driver_id/timestamp 补齐逻辑迁入 adapter。

### 5.4 Dynamic Proxy

新增 `dynamic_proxy.rs` 或在 `dynamic_driver.rs` 内先建私有 `DynamicDriverProxy`，封装：

- symbol resolution
- ABI version check
- C string read/free
- manifest ABI -> domain manifest

第一轮可以先以私有结构抽取，不改变 public export。

### 5.5 Session State

新增 `session.rs`，提供 `DriverSessionState`：

```rust
pub enum DriverSessionState {
    Created,
    Initializing,
    Ready,
    Executing,
    ShuttingDown,
    Closed,
    Failed { reason: String },
}
```

第一轮只用于 dynamic driver lifecycle 内部标记，不改变外部 `SoftwareDriver` trait。

## 6. 实施计划

### Task 1：OpenSpec 提案与基线测试

**Files:**

- Create: `openspec/changes/refactor-macaca-driver-patterns/proposal.md`
- Create: `openspec/changes/refactor-macaca-driver-patterns/design.md`
- Create: `openspec/changes/refactor-macaca-driver-patterns/tasks.md`
- Create: `openspec/changes/refactor-macaca-driver-patterns/specs/macaca-driver-core/spec.md`

- [ ] **Step 1: 编写 OpenSpec proposal**

说明为什么要重构 `macaca-driver`，明确这是 additive-first，旧接口保留，行为 1:1 还原。

- [ ] **Step 2: 编写 design**

覆盖 Factory、Command、Trace Adapter、Dynamic Proxy、Session State 的边界和权衡。

- [ ] **Step 3: 编写 tasks**

按本计划五个切片拆成可勾选任务。

- [ ] **Step 4: 编写 delta spec**

新增 `macaca-driver-core` capability，要求：

- driver creation SHALL expose canonical factory entrypoints.
- driver command execution SHALL preserve existing tool behavior.
- driver trace adaptation SHALL preserve driver identity and action names.
- dynamic ABI loading SHALL remain compatible with ABI v1.
- driver session state SHALL be observable internally without changing public `SoftwareDriver`.

- [ ] **Step 5: 验证 OpenSpec**

Run:

```bash
openspec validate refactor-macaca-driver-patterns --strict
```

Expected:

```text
Change 'refactor-macaca-driver-patterns' is valid
```

### Task 2：第一切片，新增 DriverFactory 适配层

**Files:**

- Create: `macaca/crates/macaca-driver/src/factory.rs`
- Modify: `macaca/crates/macaca-driver/src/lib.rs`
- Modify: `macaca/crates/macaca-driver/src/loader.rs`
- Test: `macaca/crates/macaca-driver/src/factory.rs`
- Test: `macaca/crates/macaca-driver/src/loader.rs`

- [ ] **Step 1: GitNexus impact**

Run before editing:

```text
gitnexus_impact(target: "DriverLoader", direction: "upstream")
gitnexus_impact(target: "DynamicDriver", direction: "upstream")
```

Report direct callers and risk.

- [ ] **Step 2: 添加 failing tests**

Tests should assert:

- `DynamicDriverFactory` preserves configured driver name.
- `DriverCreateContext::from_toml_config(None)` produces `{}`.
- `DriverLoader::load_driver` still returns `Box<dyn SoftwareDriver>` and keeps old signature.

- [ ] **Step 3: 新增 factory 原语**

Add `DriverCreateContext`, `DriverFactory`, `DynamicDriverFactory`.

- [ ] **Step 4: 迁移 loader 内部创建**

`DriverLoader::load_driver` converts TOML config into `DriverCreateContext` and calls `DynamicDriverFactory::create`.

- [ ] **Step 5: 验证**

Run:

```bash
cargo test -p macaca-driver factory loader -- --nocapture
cargo check -p macaca-web
```

### Task 3：第二切片，抽取 DriverCommand

**Files:**

- Create: `macaca/crates/macaca-driver/src/command.rs`
- Modify: `macaca/crates/macaca-driver/src/lib.rs`
- Modify: `macaca/crates/macaca-driver/src/dynamic_driver.rs`
- Test: `macaca/crates/macaca-driver/src/command.rs`

- [ ] **Step 1: GitNexus impact**

Run:

```text
gitnexus_impact(target: "DynamicTool", direction: "upstream")
```

- [ ] **Step 2: 添加 command tests**

Tests should assert:

- execute command returns action name `execute`.
- health check command returns action name `health_check`.
- command trace label includes driver name and action.

- [ ] **Step 3: 新增 DriverCommand**

Add enum variants for `Execute`, `HealthCheck`, `Shutdown`.

- [ ] **Step 4: 在 DynamicTool 内部使用 command 描述**

Do not change FFI signature. Use `DriverCommand::Execute` only to centralize labels and metadata.

- [ ] **Step 5: 验证**

Run:

```bash
cargo test -p macaca-driver command -- --nocapture
cargo test -p macaca-driver dynamic_driver -- --nocapture
```

### Task 4：第三切片，抽取 DriverTraceAdapter

**Files:**

- Create: `macaca/crates/macaca-driver/src/trace.rs`
- Modify: `macaca/crates/macaca-driver/src/lib.rs`
- Modify: `macaca/crates/macaca-driver/src/dynamic_driver.rs`
- Test: `macaca/crates/macaca-driver/src/trace.rs`

- [ ] **Step 1: GitNexus impact**

Run:

```text
gitnexus_impact(target: "execute_streaming", direction: "upstream")
```

- [ ] **Step 2: 添加 trace adapter tests**

Tests should assert:

- missing `driver_id` is filled from fallback driver name.
- existing `driver_id` is preserved.
- missing `timestamp` is filled.
- existing `timestamp` is preserved.

- [ ] **Step 3: 迁移 trampoline normalization**

Move driver_id/timestamp enrichment out of `dynamic_driver.rs` into `DriverTraceAdapter`.

- [ ] **Step 4: 验证 UI trace contract**

Run:

```bash
cargo test -p macaca-driver trace -- --nocapture
cargo test -p macaca-web delegated_driver_trace -- --nocapture
npx tsc --noEmit
```

### Task 5：第四切片，抽取 DynamicDriverProxy

**Files:**

- Create: `macaca/crates/macaca-driver/src/dynamic_proxy.rs`
- Modify: `macaca/crates/macaca-driver/src/dynamic_driver.rs`
- Modify: `macaca/crates/macaca-driver/src/lib.rs`
- Test: `macaca/crates/macaca-driver/src/dynamic_proxy.rs`

- [ ] **Step 1: GitNexus impact**

Run:

```text
gitnexus_impact(target: "DynamicDriver", direction: "upstream")
gitnexus_impact(target: "load", direction: "upstream", file_path: "macaca/crates/macaca-driver/src/dynamic_driver.rs")
```

- [ ] **Step 2: 添加 proxy unit tests**

Pure tests should cover:

- `parse_driver_type("CliSubprocess")`.
- invalid driver type returns `MacacaError::Driver`.
- manifest ABI conversion preserves name/version/capabilities/trace_event_types.

- [ ] **Step 3: 抽出 manifest conversion**

Move `DriverManifestAbi -> DriverManifest` conversion into proxy/helper.

- [ ] **Step 4: 抽出 string ownership helper**

Keep behavior identical: non-null C string is always freed even if UTF-8 decoding fails.

- [ ] **Step 5: 不改变 DynamicDriver public API**

`DynamicDriver::load(&Path, &str) -> MacacaResult<Self>` remains unchanged.

- [ ] **Step 6: 验证**

Run:

```bash
cargo test -p macaca-driver dynamic_proxy dynamic_driver -- --nocapture
cargo check -p macaca-web
```

### Task 6：第五切片，新增 DriverSessionState

**Files:**

- Create: `macaca/crates/macaca-driver/src/session.rs`
- Modify: `macaca/crates/macaca-driver/src/lib.rs`
- Modify: `macaca/crates/macaca-driver/src/dynamic_driver.rs`
- Test: `macaca/crates/macaca-driver/src/session.rs`

- [ ] **Step 1: GitNexus impact**

Run:

```text
gitnexus_impact(target: "shutdown", direction: "upstream", file_path: "macaca/crates/macaca-driver/src/dynamic_driver.rs")
gitnexus_impact(target: "health_check", direction: "upstream", file_path: "macaca/crates/macaca-driver/src/dynamic_driver.rs")
```

- [ ] **Step 2: 添加 state transition tests**

Tests should assert:

- `Created -> Initializing -> Ready` is allowed.
- `Ready -> Executing -> Ready` is allowed.
- `Ready -> ShuttingDown -> Closed` is allowed.
- `Closed -> Executing` is rejected.
- `Failed -> Ready` is rejected without explicit reset.

- [ ] **Step 3: 在 DynamicDriver 内部记录状态**

Add private state field or lightweight helper. Do not expose new public behavior in this slice.

- [ ] **Step 4: 保持 SoftwareDriver trait 不变**

No trait signature changes in this proposal.

- [ ] **Step 5: 验证**

Run:

```bash
cargo test -p macaca-driver session -- --nocapture
cargo test -p macaca-driver -- --nocapture
```

### Task 7：全链路验证与迁移检查

**Files:**

- Modify: `openspec/changes/refactor-macaca-driver-patterns/tasks.md`
- No production code changes unless verification reveals a regression.

- [ ] **Step 1: workspace compile**

Run:

```bash
cargo check
```

- [ ] **Step 2: targeted tests**

Run:

```bash
cargo test -p macaca-driver -- --nocapture
cargo test -p macaca-web delegated_driver_trace -- --nocapture
```

- [ ] **Step 3: GitNexus detect changes**

Run:

```text
gitnexus_detect_changes(scope: "all")
```

Expected:

- Changed symbols are limited to `macaca-driver` plus expected consumers/tests.
- Any HIGH/CRITICAL risk is explicitly reported before commit.

- [ ] **Step 4: commit**

Use one commit for the full approved OpenSpec implementation only after checks pass:

```bash
git add macaca/crates/macaca-driver openspec/changes/refactor-macaca-driver-patterns
git commit -m "refactor: evolve macaca-driver abstractions"
```

## 7. 不做事项

- 不改变 `SoftwareDriver` trait 签名。
- 不改变 plugin ABI version。
- 不删除 `DriverToolSet` 兼容壳。
- 不把 Claude Code、OpenCode 或任何 driver 名写死到 core driver 逻辑里。
- 不迁移 `macaca-web` 主要消费方，除非编译要求最小适配。
- 不引入新的第三方依赖。

## 8. 自检

- Spec coverage: 五个切片均覆盖 `macaca-driver.md` 中的计划。
- Placeholder scan: 无 TBD/TODO/implement later。
- Type consistency: 所有新增类型名在首次使用处定义。
- Scope check: 本计划只覆盖 `macaca-driver` core additive-first 重构，不包含上层消费方迁移。
- Behavior check: 外部 API 和 ABI 先保持不变，旧调用路径继续可用。

