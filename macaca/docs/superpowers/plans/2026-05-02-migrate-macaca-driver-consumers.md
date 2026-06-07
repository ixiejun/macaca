# Migrate macaca-driver Consumers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将上层 crate 迁移到本次基于设计模式重构后的 `macaca-driver` 原语上，让 `macaca-web` 不再手写 driver boot/reload/list/tool aggregation 细节。

**Architecture:** 采用 additive-first 迁移：先在 `macaca-driver` 增加通用 `DriverRuntime` / `DriverLoadCommand` / `DriverInventory` facade，复用已有 `DriverFactory`、`DriverRegistry::collect_tools()`、`DriverLoader::load_all()` 语义；再把 `macaca-web` 的启动加载、reload API、driver 列表和 toolkit driver tool 注入迁移到 facade。旧入口保留并 deprecated，不删除，便于后续 grep 清理。

**Tech Stack:** Rust, tokio, async_trait, macaca-driver, macaca-web, macaca-tools, OpenSpec, GitNexus.

---

## 1. 已阅读上下文

### 1.1 相关文档

- `AGENTS.md`
- `macaca/docs/design-pattern-refactor-plans/README.md`
- `macaca/docs/design-pattern-refactor-plans/refactor-order.md`
- `macaca/docs/design-pattern-refactor-plans/macaca-driver.md`
- `macaca/docs/design_patterns.md`
- `docs/superpowers/plans/2026-05-02-refactor-macaca-driver.md`
- `openspec/changes/refactor-macaca-driver-patterns/proposal.md`
- `openspec/changes/refactor-macaca-driver-patterns/design.md`
- `openspec/changes/refactor-macaca-driver-patterns/tasks.md`
- `openspec/changes/refactor-macaca-driver-patterns/specs/macaca-driver-core/spec.md`

### 1.2 当前 `macaca-driver` 新原语

- `DriverFactory`
- `DriverCreateContext`
- `DynamicDriverFactory`
- `DriverCommand`
- `DriverTraceAdapter`
- `DynamicDriverProxy`
- `DriverSessionState`
- `DriverRegistry::collect_tools()`

旧入口已保留并 deprecated：

- `DynamicDriver::load()`
- `DriverLoader::load_driver()`
- `DriverRegistry::aggregate_tools()`

### 1.3 上层真实消费代码

通过 `rg` 与 GitNexus 查询，真实上层消费集中在 `macaca-web`：

- `macaca/crates/macaca-web/src/lib.rs`
  - server 启动时直接 `DriverLoader::new()` + `load_all()`
  - 手写 driver load result loop
  - 手动调用 `SoftwareDriver::tools(driver.as_ref()).len()` 计算 tool count
  - 手动注册到 `DriverRegistry`

- `macaca/crates/macaca-web/src/routes.rs`
  - `/api/drivers/reload` 直接 `DriverLoader::new()` + `registry.clear()` + `load_all()`
  - 手写 success/error response 聚合
  - 手动调用 `SoftwareDriver::tools(driver.as_ref()).len()`

- `macaca/crates/macaca-web/src/framework_toolkit.rs`
  - `build_toolkit()` 从 `state.driver_registry.collect_tools().await` 聚合 tools
  - 这是上一轮已经迁移过的最小 consumer path

- `macaca/crates/macaca-web/src/state.rs`
  - `AppState` 持有 `Arc<DriverRegistry>`

- `macaca/crates/macaca-integration-tests/tests/fullstack_autodev.rs`
  - 只有关于外部 driver plugin 的注释，没有直接 consumer 调用

其他 crate：

- `macaca-integration-tests` 依赖 `macaca-driver`，但当前没有直接调用 driver API
- `macaca-framework` / `macaca-runtime` 消费的是 `macaca-tools::TraceEvent` 或 `AgentExecutionEvent::DriverTrace`，不是 `macaca-driver` API

## 2. Superpowers Brainstorm

### 方案 A：只替换剩余直接旧 API 调用

做法：

- 保持 `macaca-web` 当前 orchestration 结构
- 只把显式 deprecated 调用换成新方法
- 继续让 web 自己负责 loader、registry、reload response 和 tool count

优点：

- 改动最小
- 风险最低
- 可快速验证

风险：

- 迁移价值有限
- `macaca-web` 继续知道太多 driver lifecycle 细节
- 后续 driver runtime 支持更多来源、隔离加载、健康检查、权限策略时，web 仍会被迫继续膨胀

结论：

- 不推荐作为本轮目标，因为当前 grep 已显示 deprecated 调用基本清空，只做这个等于没有真正迁移上层架构。

### 方案 B：在 `macaca-driver` 新增通用 runtime facade，再迁移 web

做法：

- 在 `macaca-driver` 新增：
  - `DriverLoadCommand`
  - `DriverLoadReport`
  - `DriverRuntime`
  - `DriverInventoryItem`
- `DriverRuntime` 组合 `DriverLoader` + `Arc<DriverRegistry>`
- 启动加载、reload、list、collect tools 都走 facade
- `macaca-web` 只持有和调用 facade，少量保留 `DriverRegistry` 兼容字段或改成通过 facade 获取 registry

优点：

- 符合 Facade + Command + Factory + Adapter 方向
- web 入口变薄，driver lifecycle 回到 driver crate
- 保持通用，不硬编码 application、workflow、driver name
- 后续可以自然接入 driver health monitor、isolated load、runtime policy、driver source provider

风险：

- 比方案 A 多一个新 public facade，需要 OpenSpec 约束清楚
- `DriverRuntime` 内部会持有 registry，需要避免和 `AppState.driver_registry` 形成双源状态
- `/api/drivers/reload` 的 clear-then-load 语义必须 1:1 保留，否则可能影响运行时可见 tools

结论：

- 推荐。它是最小但真正有架构收益的迁移。

### 方案 C：彻底替换 `AppState.driver_registry` 为 `DriverRuntime`

做法：

- `AppState` 不再暴露 `driver_registry`
- 所有 web 代码只通过 `driver_runtime`
- routes、toolkit、startup 全部迁移到 runtime facade

优点：

- 边界最干净
- 上层完全不再依赖 registry 实现

风险：

- `AppState` 字段变更会影响较多测试和初始化代码
- 如果后续还有未索引外部调用，会一次性破坏面较大
- 不符合“小步、可回滚”的优先原则

结论：

- 不作为第一轮实施。可以作为后续第二轮 clean-up。

## 3. 推荐方案

采用方案 B：新增 `DriverRuntime` facade，并迁移主要 web consumer。

设计模式对应：

- Facade：`DriverRuntime` 对外统一提供 load/reload/list/collect tools。
- Command：`DriverLoadCommand` 描述启动加载和 reload 加载意图。
- Factory：继续复用 `DriverLoader` -> `DynamicDriverFactory` 创建路径。
- Adapter：`DriverInventoryItem` 把 `DriverManifest` + tool count 适配为上层展示数据。
- Composite：driver tools 仍以 `Vec<Box<dyn Tool>>` 交给 framework toolkit 注册，不引入新聚合实现。

范围控制：

- 不删除 `DriverRegistry`
- 不删除 `DriverLoader`
- 不改变 `/api/drivers/reload` 返回 JSON
- 不改变 driver tools 注入 timing
- 不改变 `SoftwareDriver` trait
- 不新增依赖

## 4. 风险与缓解

### 4.1 Registry 双源风险

风险：

- 如果 `AppState` 同时持有 `driver_registry` 和 `driver_runtime`，但二者不是同一个 registry，会导致 reload 与 build_toolkit 看到不同状态。

缓解：

- `DriverRuntime::new(drivers_dir, registry)` 必须接收外部传入的 `Arc<DriverRegistry>`。
- `AppState` 第一轮可以继续保留 `driver_registry`，但 `driver_runtime.registry()` 返回同一个 Arc。
- 测试用 `Arc::ptr_eq` 或行为测试验证同源。

### 4.2 Reload 行为变化风险

风险：

- 现有 reload 是 `clear()` 后加载，失败 driver 不保留旧实例。

缓解：

- 第一轮 `DriverRuntime::reload()` 必须保持 clear-then-load。
- 不实现“失败回滚到旧 registry”的高级行为。

### 4.3 Tool count 副作用风险

风险：

- 现在 tool count 通过 `SoftwareDriver::tools(driver.as_ref()).len()` 计算，动态 driver 的 `tools()` 会读取 tool definitions 并创建 tool proxy。

缓解：

- 第一轮保持相同行为。
- `DriverLoadReport` 中记录 `tool_count`，web 不再手动调用 trait 方法。

### 4.4 Concurrency 风险

风险：

- `build_toolkit()` 可能在 reload 期间 collect tools。

缓解：

- 继续复用 `DriverRegistry` 内部 `RwLock`。
- 不扩大锁持有范围。
- `reload()` 仍然用现有 `clear()` + `register()` 序列，行为与现状一致。

### 4.5 Scope 膨胀风险

风险：

- 顺手加入 health monitor、driver source provider、driver permission，会把小迁移变成大功能。

缓解：

- 本轮只做 migration facade，不做新能力。
- health/status 仍由现有接口保持。

## 5. 文件结构计划

### 5.1 新增文件

- `macaca/crates/macaca-driver/src/runtime.rs`
  - 定义 `DriverRuntime`
  - 组合 `PathBuf drivers_dir` 与 `Arc<DriverRegistry>`
  - 提供 `load_all()`, `reload()`, `list_inventory()`, `collect_tools()`, `registry()`

- `macaca/crates/macaca-driver/src/load_command.rs`
  - 定义 `DriverLoadCommand`
  - 定义 `DriverLoadReport`
  - 定义 `DriverLoadEntry`
  - 将 `DriverLoadResult` 转换为报告项

### 5.2 修改文件

- `macaca/crates/macaca-driver/src/lib.rs`
  - export `DriverRuntime`, `DriverLoadCommand`, `DriverLoadReport`, `DriverLoadEntry`, `DriverInventoryItem`

- `macaca/crates/macaca-driver/src/registry.rs`
  - 可选：新增 `registry_snapshot()` 或 `inventory()`，但优先不动，避免扩大影响

- `macaca/crates/macaca-web/src/lib.rs`
  - 启动加载迁移为 `DriverRuntime::load_all()`
  - `AppState` 构建时放入同源 registry/runtime

- `macaca/crates/macaca-web/src/state.rs`
  - 新增 `pub driver_runtime: Arc<macaca_driver::DriverRuntime>`
  - 保留 `driver_registry` 兼容字段，避免一次性大范围迁移

- `macaca/crates/macaca-web/src/routes.rs`
  - `/api/drivers/reload` 迁移为 `state.driver_runtime.reload().await`
  - `get_drivers` 迁移为 `state.driver_runtime.list_inventory().await`

- `macaca/crates/macaca-web/src/framework_toolkit.rs`
  - `build_toolkit()` 从 `state.driver_runtime.collect_tools().await` 获取 driver tools

### 5.3 OpenSpec

- 新增 `openspec/changes/migrate-driver-consumers-to-runtime-primitives/`
  - `proposal.md`
  - `design.md`
  - `tasks.md`
  - `specs/macaca-driver-core/spec.md`

## 6. 执行计划

### Task 1: OpenSpec 迁移提案

**Files:**

- Create: `openspec/changes/migrate-driver-consumers-to-runtime-primitives/proposal.md`
- Create: `openspec/changes/migrate-driver-consumers-to-runtime-primitives/design.md`
- Create: `openspec/changes/migrate-driver-consumers-to-runtime-primitives/tasks.md`
- Create: `openspec/changes/migrate-driver-consumers-to-runtime-primitives/specs/macaca-driver-core/spec.md`

- [ ] **Step 1: 编写 proposal**

写明本轮目标是“迁移上层 consumer”，不是继续重构 driver ABI。

必须包含：

```markdown
## Why

`macaca-web` still manually orchestrates driver loading, reloading, inventory listing, and tool aggregation. This keeps driver lifecycle knowledge in the web entry layer instead of the driver infrastructure crate.

## What Changes

- Add a driver runtime facade in `macaca-driver`.
- Move startup load and reload orchestration behind that facade.
- Move driver inventory and tool aggregation consumer paths to that facade.
- Keep legacy registry/loader APIs as deprecated compatibility wrappers.
```

- [ ] **Step 2: 编写 design**

设计中明确：

```markdown
## Decision

Use `DriverRuntime` as a Facade over `DriverLoader` and `DriverRegistry`.
Use `DriverLoadCommand` as the command object for load/reload intent.
Keep `DriverRegistry` as the underlying state holder for this slice.
```

- [ ] **Step 3: 编写 delta spec**

至少包含这些 requirements：

```markdown
### Requirement: Upper-layer driver lifecycle SHALL use runtime facade

### Requirement: Driver reload SHALL preserve existing clear-then-load behavior

### Requirement: Driver inventory SHALL be exposed without web manually calling SoftwareDriver::tools

### Requirement: Driver tools SHALL be collected through the runtime facade
```

- [ ] **Step 4: 验证 OpenSpec**

Run:

```bash
openspec validate migrate-driver-consumers-to-runtime-primitives --strict
```

Expected:

```text
Change 'migrate-driver-consumers-to-runtime-primitives' is valid
```

### Task 2: Add DriverRuntime facade

**Files:**

- Create: `macaca/crates/macaca-driver/src/runtime.rs`
- Create: `macaca/crates/macaca-driver/src/load_command.rs`
- Modify: `macaca/crates/macaca-driver/src/lib.rs`
- Test: `macaca/crates/macaca-driver/src/runtime.rs`

- [ ] **Step 1: Run GitNexus impact**

Run impact before editing:

```text
gitnexus_impact(target: "DriverRegistry", direction: "upstream")
gitnexus_impact(target: "DriverLoader", direction: "upstream")
```

Expected:

- Report direct callers and affected flows.
- Stop and warn if risk is HIGH or CRITICAL.

- [ ] **Step 2: Add load command types**

Create `load_command.rs`:

```rust
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum DriverLoadCommand {
    LoadAll,
    Reload,
}

#[derive(Debug, Clone)]
pub struct DriverLoadReport {
    pub command: DriverLoadCommand,
    pub loaded: usize,
    pub failed: usize,
    pub entries: Vec<DriverLoadEntry>,
}

#[derive(Debug, Clone)]
pub struct DriverLoadEntry {
    pub name: String,
    pub path: PathBuf,
    pub status: DriverLoadStatus,
    pub tool_count: Option<usize>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverLoadStatus {
    Loaded,
    Failed,
}
```

- [ ] **Step 3: Add runtime facade**

Create `runtime.rs`:

```rust
use std::path::{Path, PathBuf};
use std::sync::Arc;

use macaca_tools::Tool;

use crate::driver::DriverManifest;
use crate::load_command::{DriverLoadCommand, DriverLoadEntry, DriverLoadReport, DriverLoadStatus};
use crate::loader::DriverLoader;
use crate::registry::DriverRegistry;

#[derive(Debug, Clone)]
pub struct DriverInventoryItem {
    pub manifest: DriverManifest,
    pub tool_count: usize,
}

pub struct DriverRuntime {
    drivers_dir: PathBuf,
    registry: Arc<DriverRegistry>,
}

impl DriverRuntime {
    pub fn new(drivers_dir: impl Into<PathBuf>, registry: Arc<DriverRegistry>) -> Self {
        Self {
            drivers_dir: drivers_dir.into(),
            registry,
        }
    }

    pub fn drivers_dir(&self) -> &Path {
        &self.drivers_dir
    }

    pub fn registry(&self) -> Arc<DriverRegistry> {
        Arc::clone(&self.registry)
    }

    pub async fn load_all(&self) -> DriverLoadReport {
        self.load_with_command(DriverLoadCommand::LoadAll, false).await
    }

    pub async fn reload(&self) -> DriverLoadReport {
        self.load_with_command(DriverLoadCommand::Reload, true).await
    }

    async fn load_with_command(&self, command: DriverLoadCommand, clear_first: bool) -> DriverLoadReport {
        if clear_first {
            self.registry.clear().await;
        }

        let loader = DriverLoader::new(&self.drivers_dir);
        let results = loader.load_all();
        let mut entries = Vec::new();
        let mut loaded = 0usize;
        let mut failed = 0usize;

        for result in results {
            match result.result {
                Ok(driver) => {
                    let tool_count = crate::SoftwareDriver::tools(driver.as_ref()).len();
                    self.registry.register(driver).await;
                    loaded += 1;
                    entries.push(DriverLoadEntry {
                        name: result.name,
                        path: result.path,
                        status: DriverLoadStatus::Loaded,
                        tool_count: Some(tool_count),
                        error: None,
                    });
                }
                Err(error) => {
                    failed += 1;
                    entries.push(DriverLoadEntry {
                        name: result.name,
                        path: result.path,
                        status: DriverLoadStatus::Failed,
                        tool_count: None,
                        error: Some(error),
                    });
                }
            }
        }

        DriverLoadReport {
            command,
            loaded,
            failed,
            entries,
        }
    }

    pub async fn list_inventory(&self) -> Vec<DriverInventoryItem> {
        self.registry
            .list_drivers_with_tools()
            .await
            .into_iter()
            .map(|(manifest, tool_count)| DriverInventoryItem {
                manifest,
                tool_count,
            })
            .collect()
    }

    pub async fn collect_tools(&self) -> Vec<Box<dyn Tool>> {
        self.registry.collect_tools().await
    }
}
```

- [ ] **Step 4: Export runtime types**

Modify `lib.rs`:

```rust
pub mod load_command;
pub mod runtime;

pub use load_command::{DriverLoadCommand, DriverLoadEntry, DriverLoadReport, DriverLoadStatus};
pub use runtime::{DriverInventoryItem, DriverRuntime};
```

- [ ] **Step 5: Add focused tests**

Add tests in `runtime.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_uses_shared_registry() {
        let registry = Arc::new(DriverRegistry::new());
        let runtime = DriverRuntime::new("/tmp/drivers", Arc::clone(&registry));
        assert!(Arc::ptr_eq(&registry, &runtime.registry()));
        assert_eq!(runtime.drivers_dir(), Path::new("/tmp/drivers"));
    }
}
```

- [ ] **Step 6: Run driver tests**

Run:

```bash
cargo test -p macaca-driver -- --nocapture
```

Expected:

```text
test result: ok
```

### Task 3: Migrate web startup load to DriverRuntime

**Files:**

- Modify: `macaca/crates/macaca-web/src/lib.rs`
- Modify: `macaca/crates/macaca-web/src/state.rs`

- [ ] **Step 1: Run GitNexus impact**

Run:

```text
gitnexus_impact(target: "main", direction: "upstream")
gitnexus_impact(target: "AppState", direction: "upstream")
```

If `main` is ambiguous, use `gitnexus_context` to pick the web server startup symbol.

- [ ] **Step 2: Add runtime field to AppState**

Modify `state.rs`:

```rust
use macaca_driver::{DriverRegistry, DriverRuntime};

pub struct AppState {
    ...
    pub driver_registry: Arc<DriverRegistry>,
    pub driver_runtime: Arc<DriverRuntime>,
    ...
}
```

Keep `driver_registry` in this slice for compatibility and to reduce blast radius.

- [ ] **Step 3: Build one shared registry/runtime in startup**

In `lib.rs`, replace direct startup load loop with:

```rust
let driver_registry = Arc::new(macaca_driver::DriverRegistry::new());
let driver_runtime = Arc::new(macaca_driver::DriverRuntime::new(
    drivers_dir.clone(),
    Arc::clone(&driver_registry),
));
if config.drivers.auto_load {
    let report = driver_runtime.load_all().await;
    for entry in &report.entries {
        match entry.status {
            macaca_driver::DriverLoadStatus::Loaded => {
                info!(
                    name = %entry.name,
                    tools = entry.tool_count.unwrap_or_default(),
                    "External driver loaded"
                );
            }
            macaca_driver::DriverLoadStatus::Failed => {
                error!(
                    name = %entry.name,
                    error = %entry.error.as_deref().unwrap_or("unknown error"),
                    "Failed to load external driver"
                );
            }
        }
    }
}
```

- [ ] **Step 4: Insert runtime into AppState**

When constructing `AppState`, set:

```rust
driver_registry: Arc::clone(&driver_registry),
driver_runtime: Arc::clone(&driver_runtime),
```

- [ ] **Step 5: Compile web**

Run:

```bash
cargo check -p macaca-web -p macaca-driver
```

Expected:

```text
Finished `dev` profile
```

### Task 4: Migrate driver routes

**Files:**

- Modify: `macaca/crates/macaca-web/src/routes.rs`

- [ ] **Step 1: Run GitNexus impact**

Run:

```text
gitnexus_impact(target: "get_drivers", direction: "upstream")
gitnexus_impact(target: "reload_drivers", direction: "upstream")
```

- [ ] **Step 2: Migrate get_drivers**

Replace registry direct listing with:

```rust
let driver_info = state.driver_runtime.list_inventory().await;
let drivers: Vec<DriverInfo> = driver_info
    .into_iter()
    .map(|item| DriverInfo {
        name: item.manifest.name,
        version: item.manifest.version,
        driver_type: format!("{:?}", item.manifest.driver_type),
        description: item.manifest.description,
        capabilities: item.manifest.capabilities,
        tools_count: item.tool_count,
    })
    .collect();
```

- [ ] **Step 3: Migrate reload_drivers**

Replace manual loader/clear/register loop with:

```rust
let report = state.driver_runtime.reload().await;

let results = report
    .entries
    .into_iter()
    .map(|entry| DriverReloadResult {
        name: entry.name,
        status: match entry.status {
            macaca_driver::DriverLoadStatus::Loaded => "ok".to_string(),
            macaca_driver::DriverLoadStatus::Failed => "error".to_string(),
        },
        error: entry.error,
    })
    .collect();

Ok(Json(DriverReloadResponse {
    loaded: report.loaded,
    failed: report.failed,
    results,
}))
```

- [ ] **Step 4: Compile web**

Run:

```bash
cargo check -p macaca-web -p macaca-driver
```

Expected:

```text
Finished `dev` profile
```

### Task 5: Migrate toolkit driver tool collection

**Files:**

- Modify: `macaca/crates/macaca-web/src/framework_toolkit.rs`

- [ ] **Step 1: Run GitNexus impact**

Run:

```text
gitnexus_impact(target: "build_toolkit", direction: "upstream")
```

Expected risk may be HIGH/CRITICAL because this touches agent construction. Report it before editing.

- [ ] **Step 2: Replace registry collect with runtime collect**

Replace:

```rust
let driver_tools = state.driver_registry.collect_tools().await;
```

with:

```rust
let driver_tools = state.driver_runtime.collect_tools().await;
```

- [ ] **Step 3: Compile web**

Run:

```bash
cargo check -p macaca-web -p macaca-driver
```

Expected:

```text
Finished `dev` profile
```

### Task 6: Deprecate remaining direct upper-layer lifecycle entrypoints

**Files:**

- Modify: `macaca/crates/macaca-driver/src/loader.rs`
- Modify: `macaca/crates/macaca-driver/src/registry.rs`

- [ ] **Step 1: Mark `DriverLoader::load_all` deprecated**

Add:

```rust
#[deprecated(note = "use DriverRuntime::load_all() or DriverRuntime::reload()")]
pub fn load_all(&self) -> Vec<DriverLoadResult> {
    ...
}
```

Keep internal `DriverRuntime` usage contained. If `DriverRuntime` still needs non-deprecated access, add a crate-visible method:

```rust
pub(crate) fn load_all_internal(&self) -> Vec<DriverLoadResult> {
    ...
}
```

and let deprecated `load_all()` call it.

- [ ] **Step 2: Mark direct registry lifecycle methods only if safe**

Do not deprecate `register`, `clear`, or `list_drivers_with_tools` yet if `DriverRuntime` uses them and tests need them.

Only deprecate consumer-facing aggregate helpers that now have facade replacements:

```rust
#[deprecated(note = "use DriverRuntime::collect_tools()")]
pub async fn collect_tools(&self) -> Vec<Box<dyn Tool>> {
    ...
}
```

If this creates excessive warnings inside `macaca-driver`, defer registry deprecation to the next migration slice and document it in OpenSpec tasks.

- [ ] **Step 3: Deprecated call containment grep**

Run:

```bash
rg -n "DriverLoader::new|\\.load_all\\(|\\.load_driver\\(|\\.collect_tools\\(|\\.aggregate_tools\\(|SoftwareDriver::tools" macaca/crates --glob '*.rs'
```

Expected:

- `macaca-driver` internal compatibility wrappers may remain.
- `macaca-web` should not directly call `DriverLoader::new`, `.load_all()`, `.load_driver()`, `SoftwareDriver::tools`, or `state.driver_registry.collect_tools()`.

### Task 7: Verification and change detection

**Files:**

- Modify: `openspec/changes/migrate-driver-consumers-to-runtime-primitives/tasks.md`

- [ ] **Step 1: Validate OpenSpec**

Run:

```bash
openspec validate migrate-driver-consumers-to-runtime-primitives --strict
```

Expected:

```text
Change 'migrate-driver-consumers-to-runtime-primitives' is valid
```

- [ ] **Step 2: Run driver tests**

Run:

```bash
cargo test -p macaca-driver -- --nocapture
```

Expected:

```text
test result: ok
```

- [ ] **Step 3: Run targeted checks**

Run:

```bash
cargo check -p macaca-driver -p macaca-web -p macaca-integration-tests
```

Expected:

```text
Finished `dev` profile
```

- [ ] **Step 4: Run workspace check**

Run:

```bash
cargo check
```

Expected:

```text
Finished `dev` profile
```

- [ ] **Step 5: Run GitNexus detect changes**

Run:

```text
gitnexus_detect_changes(scope: "all")
```

Expected:

- Affected processes should be limited to driver reload/startup/toolkit construction flows.
- Any HIGH/CRITICAL risk must be reported with affected d=1 callers.

- [ ] **Step 6: Update tasks**

Mark completed OpenSpec tasks as checked only after commands pass.

## 7. Success Criteria

- `macaca-web` no longer manually orchestrates driver load/reload loops.
- `macaca-web` no longer manually calls `SoftwareDriver::tools(driver.as_ref()).len()` for load reports.
- `build_toolkit()` obtains driver tools through `DriverRuntime`.
- `/api/drivers/reload` response stays JSON-compatible.
- Existing clear-then-load reload semantics stay unchanged.
- Existing startup auto-load behavior stays unchanged.
- No application name, workflow name, or driver name is hardcoded.
- `openspec validate` passes.
- `cargo test -p macaca-driver` passes.
- workspace `cargo check` passes.

## 8. Self-Review

- Spec coverage: plan includes OpenSpec proposal/design/tasks/spec before implementation.
- Scope: only migrates upper consumer paths to driver runtime facade; no ABI or `SoftwareDriver` trait change.
- Placeholder scan: no TBD/TODO/implement later placeholders.
- Type consistency: `DriverRuntime`, `DriverLoadCommand`, `DriverLoadReport`, `DriverLoadEntry`, `DriverInventoryItem` are defined before use.
- Risk coverage: reload semantics, shared registry source, tool count behavior, and build_toolkit high-risk path are explicitly handled.
