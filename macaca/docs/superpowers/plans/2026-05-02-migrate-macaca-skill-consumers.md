# Migrate macaca-skill Consumers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将上层 crate 从 `macaca-skill` 的旧直接接口迁移到本次基于设计模式重构后的 skill primitives，消除非测试路径的 deprecated 调用，并保持 skill catalog、skill-backed MCP、executable skill tools、session snapshot 行为 1:1 不变。

**Architecture:** 采用 additive-first 消费方迁移：先把上一轮 `refactor-macaca-skill-patterns` OpenSpec 变更移到正确的仓库根 `openspec/` 并补齐校验，再在 `macaca-skill` 内增加薄 Facade/Builder 入口承接上层调用，最后逐步迁移 `macaca-web`、`macaca-app` 和 integration tests。迁移期间旧 API 保留并继续 deprecated，最终 grep 只允许 `macaca-skill` 内部兼容测试命中。

**Tech Stack:** Rust, Tokio, macaca-skill, macaca-web, macaca-app, macaca-integration-tests, OpenSpec, GitNexus.

---

## Current Context

当前 `macaca-skill` 已新增这些设计模式 primitive：

- `SkillPolicyChain` / `SkillExposurePolicy`：Strategy + Chain of Responsibility，用于 metadata gating。
- `SkillSourceSet` / `SkillSource`：Abstract Factory + Registry，用于统一 skill source 顺序。
- `SkillRegistrySnapshot`：Memento，用于 executable skill registry snapshot/reload。
- `SkillToolAdapter` / `SkillRuntimeProxy`：Adapter + Proxy，用于 executable skill tool exposure。
- `SkillRuntimeHandle` / `SkillRuntimeState`：State，用于 provision lifecycle。

当前上层直接消费旧接口的位置：

- `macaca-web/src/lib.rs`
  - `SkillCatalog::new()` + `catalog.load_from_directory(&dir)` 用于启动时加载 knowledge skill catalog。
  - `SkillRegistry::new()` + `load_from_directory(dir)` + `instantiate_all_tools()` 用于启动时加载 executable skill tools。
- `macaca-web/src/framework_runner.rs`
  - 直接构造 `SkillRuntimeOptions` 并调用 `SkillRuntime.build_snapshot(...)`。
  - 内部重复实现 `resolve_agent_skill_policy(...)`。
- `macaca-web/src/skill_mcp.rs`
  - 直接构造 `SkillRuntimeOptions` 并调用 `SkillRuntime.build_snapshot(...)`。
  - 内部重复实现 `resolve_agent_skill_policy(...)`。
- `macaca-web/src/routes.rs`
  - `/api/apps/{app_id}/skills` 直接构造 `SkillRuntimeOptions` 并调用 `SkillRuntime.build_snapshot(...)`。
- `macaca-app/src/skills.rs`
  - `SkillLoader` 自己维护 global/app skill dirs、exist/path/list 逻辑，和 `SkillSourceSet` 的 source 顺序存在重复。
- `macaca-integration-tests/tests/fullstack_autodev.rs`
  - 直接调用 `SkillRegistry::load_from_directory(...)` 和 `instantiate_tool(...)`。
- `macaca-integration-tests/tests/live_fullstack_autodev.rs`
  - 直接调用 `SkillCatalog::load_from_directory(...)`。

当前 OpenSpec 注意事项：

- 项目真实 OpenSpec 根目录是 `/Users/quantum/Code/dev/agent/openspec`。
- 上一轮误把 `refactor-macaca-skill-patterns` 写到了 `/Users/quantum/Code/dev/agent/macaca/openspec`。
- 本次实现前必须先把该变更迁移到根 `openspec/changes/refactor-macaca-skill-patterns`，再创建本次迁移提案。

## Superpowers Brainstorm

### Option A: 只改 web 的 deprecated 调用

把 `macaca-web/src/lib.rs` 的 `SkillRegistry::load_from_directory` 和 `instantiate_all_tools` 改成直接使用 `SkillToolAdapter`，其他 `SkillRuntime.build_snapshot` 直调保持不变。

Trade-offs:

- 优点：改动最少，能快速减少 deprecated warning。
- 缺点：`framework_runner`、`skill_mcp`、`routes` 仍然各自拼 `SkillRuntimeOptions` 和 policy，重复逻辑继续存在；`macaca-app::SkillLoader` 仍然维护一套 source 规则。
- 结论：不推荐。它只是清 warning，不是真正迁移到设计模式 primitive。

### Option B: 在 `macaca-skill` 增加 thin consumer Facade，再迁移 web/app/tests

在 `macaca-skill` 中新增面向消费方的薄入口：

- `SkillSnapshotRequest` / `SkillSnapshotBuilder`：Builder，封装 agent/workspace/app/policy/limits。
- `SkillRuntimeFacade`：Facade，统一 `build_snapshot` 和 snapshot serialization 入口。
- `ExecutableSkillToolSet`：Facade + Adapter，统一从目录加载 YAML executable skills 并输出 `Vec<Box<dyn Tool>>`，内部使用 `SkillRegistrySnapshot` + `SkillToolAdapter`。
- `SkillCatalogSourceView`：Facade，统一 knowledge skill source inventory，供 `macaca-app::SkillLoader` 和 web startup 使用。

然后迁移上层：

- `macaca-web` 所有 snapshot 构建统一走 `SkillSnapshotRequest`。
- `macaca-web` startup executable skills 统一走 `ExecutableSkillToolSet`。
- `macaca-app::SkillLoader` 改为基于 `SkillSourceSet` 生成 source inventory。
- integration tests 改为验证新入口，旧接口只在 `macaca-skill` 自身兼容测试中保留。

Trade-offs:

- 优点：迁移边界清晰；改动可切片；不会把 web 内部状态强塞进 skill crate；能消除非测试路径 deprecated 调用。
- 缺点：需要新增少量 facade 类型，短期会和旧 API 并存。
- 结论：推荐。它符合小步、可回滚、1:1 行为还原。

### Option C: 直接把 skill snapshot/policy 与 app manifest resolver 下沉到 `macaca-skill`

让 `macaca-skill` 直接依赖 app manifest 类型，提供 `build_snapshot_for_app_agent(...)` 这种更高层 API。

Trade-offs:

- 优点：上层调用最少。
- 缺点：会让底层 `macaca-skill` 反向依赖 `macaca-app` 语义，破坏当前 crate 依赖方向；未来 skill 生态无法独立复用。
- 结论：不采用。`macaca-skill` 是基础设施层，不应该理解 application manifest。

## Recommended Design

采用 Option B。

迁移原则：

- 不把 app/workflow/driver/application name 写死到 `macaca-skill`。
- 不改变 `SkillRuntime::build_snapshot` 输出字段、过滤 reason、source precedence、prompt 格式。
- 不改变 executable skill tool 执行结果 JSON shape：`stdout`、`stderr`、`exit_code`、`command`。
- 不改变 skill-backed MCP 的 MCP runtime ownership，`skill_mcp.rs` 仍只把 visible snapshot 转为 OS MCP definitions。
- 所有新 Facade 都是 thin wrapper，只组合已有 primitive，不引入新外部依赖。

## File Map

### New or modified `macaca-skill`

- Create: `macaca/crates/macaca-skill/src/request.rs`
  - Defines `SkillSnapshotRequest` and `SkillSnapshotRequestBuilder`.
  - Responsibility: Builder for snapshot inputs.
- Create: `macaca/crates/macaca-skill/src/facade.rs`
  - Defines `SkillRuntimeFacade`, `ExecutableSkillToolSet`, `SkillCatalogSourceView`.
  - Responsibility: stable consumer-facing APIs.
- Modify: `macaca/crates/macaca-skill/src/lib.rs`
  - Export new request/facade types.
- Modify: `macaca/crates/macaca-skill/src/catalog.rs`
  - Add additive `load_from_sources(...)` helper if needed.
- Modify: `macaca/crates/macaca-skill/src/registry.rs`
  - Keep deprecated APIs, but add non-deprecated `load_executable_definitions_from_directory(...)` or facade-private helper if needed.

### Modified upper consumers

- Modify: `macaca/crates/macaca-web/src/lib.rs`
  - Replace executable skill startup path with `ExecutableSkillToolSet`.
  - Replace startup catalog load with `SkillCatalogSourceView` or `SkillCatalog::load_from_sources`.
- Modify: `macaca/crates/macaca-web/src/framework_runner.rs`
  - Replace direct `SkillRuntime.build_snapshot(...)` with `SkillRuntimeFacade::build_snapshot(SkillSnapshotRequest)`.
- Modify: `macaca/crates/macaca-web/src/skill_mcp.rs`
  - Replace direct snapshot construction with the same facade/request path.
- Modify: `macaca/crates/macaca-web/src/routes.rs`
  - Replace skill status snapshot construction with the same facade/request path.
- Modify: `macaca/crates/macaca-app/src/skills.rs`
  - Replace hand-rolled source directory logic with `SkillSourceSet`.
- Modify: `macaca/crates/macaca-integration-tests/tests/fullstack_autodev.rs`
  - Replace deprecated executable skill tests with `ExecutableSkillToolSet`.
- Modify: `macaca/crates/macaca-integration-tests/tests/live_fullstack_autodev.rs`
  - Replace direct catalog load with source/facade helper if needed.

### OpenSpec

- Move/Fix: `macaca/openspec/changes/refactor-macaca-skill-patterns/*`
  - Move into `openspec/changes/refactor-macaca-skill-patterns/*`.
- Create: `openspec/changes/migrate-skill-consumers-to-pattern-primitives/*`
  - Proposal/design/tasks/spec for this migration.

## Task 1: Repair OpenSpec location and create migration proposal

**Files:**
- Move: `macaca/openspec/changes/refactor-macaca-skill-patterns` -> `openspec/changes/refactor-macaca-skill-patterns`
- Create: `openspec/changes/migrate-skill-consumers-to-pattern-primitives/proposal.md`
- Create: `openspec/changes/migrate-skill-consumers-to-pattern-primitives/design.md`
- Create: `openspec/changes/migrate-skill-consumers-to-pattern-primitives/tasks.md`
- Create: `openspec/changes/migrate-skill-consumers-to-pattern-primitives/specs/macaca-skill-consumers/spec.md`

- [ ] **Step 1: Move misplaced OpenSpec change**

Run:

```bash
cd /Users/quantum/Code/dev/agent
mkdir -p openspec/changes
mv macaca/openspec/changes/refactor-macaca-skill-patterns openspec/changes/refactor-macaca-skill-patterns
rmdir macaca/openspec/changes macaca/openspec 2>/dev/null || true
```

Expected:

```text
openspec/changes/refactor-macaca-skill-patterns/proposal.md exists
macaca/openspec no longer exists or is empty
```

- [ ] **Step 2: Validate repaired refactor proposal**

Run:

```bash
openspec validate refactor-macaca-skill-patterns --strict
```

Expected:

```text
Change 'refactor-macaca-skill-patterns' is valid
```

- [ ] **Step 3: Create migration proposal**

Create `openspec/changes/migrate-skill-consumers-to-pattern-primitives/proposal.md`:

```markdown
# Change: Migrate macaca-skill consumers to pattern primitives

## Why
`macaca-skill` now exposes design-pattern primitives for policy, source discovery, registry snapshots, tool adapters, and lifecycle handles. Upper crates still call deprecated direct APIs and duplicate snapshot/source construction logic, which prevents clean migration and keeps skill behavior scattered across web/app/test layers.

## What Changes
- Add thin consumer-facing skill request/facade APIs in `macaca-skill`.
- Migrate `macaca-web` skill startup, snapshot construction, skill status, and skill-backed MCP paths to those APIs.
- Migrate `macaca-app::SkillLoader` source inventory to `SkillSourceSet`.
- Migrate integration tests away from deprecated `SkillRegistry` and `SkillTool` constructors.
- Keep deprecated APIs in `macaca-skill` for compatibility, but remove non-test upper-crate usages.

## Impact
- Affected specs: macaca-skill-consumers
- Affected code: `crates/macaca-skill`, `crates/macaca-web`, `crates/macaca-app`, `crates/macaca-integration-tests`
- Runtime behavior must remain 1:1 for visible skills, filtered skills, snapshot persistence, skill-backed MCP, and executable skill tool execution.
```

- [ ] **Step 4: Create migration design**

Create `openspec/changes/migrate-skill-consumers-to-pattern-primitives/design.md`:

```markdown
## Context

The previous `macaca-skill` refactor introduced primitive building blocks but intentionally did not migrate upper consumers. This change migrates consumers without changing behavior.

## Goals

- Remove deprecated `SkillRegistry::load_from_directory`, `SkillRegistry::instantiate_tool`, `SkillRegistry::instantiate_all_tools`, and `SkillTool::new` usage from upper crates.
- Centralize snapshot input construction behind `SkillSnapshotRequest`.
- Keep skill policy resolution in upper crates because `macaca-skill` must not depend on app manifest types.
- Keep MCP lifecycle in Agent OS MCP runtime.

## Non-Goals

- Do not remove deprecated APIs.
- Do not change skill metadata schema.
- Do not implement marketplace install/update.
- Do not change application-specific manifests.

## Decisions

- `SkillRuntimeFacade` accepts only generic paths, policy, limits, and agent identity.
- `macaca-web` remains responsible for resolving app/agent skill policy from application manifest.
- `ExecutableSkillToolSet` produces the same `Box<dyn macaca_tools::Tool>` values as the old startup path, but internally uses `SkillToolAdapter`.
- Deprecated APIs remain callable inside `macaca-skill` compatibility tests only.
```

- [ ] **Step 5: Create migration tasks**

Create `openspec/changes/migrate-skill-consumers-to-pattern-primitives/tasks.md`:

```markdown
## 1. OpenSpec
- [ ] 1.1 Move misplaced `refactor-macaca-skill-patterns` change to root `openspec/changes`.
- [ ] 1.2 Add migration proposal, design, tasks, and delta spec.
- [ ] 1.3 Validate both skill OpenSpec changes with `--strict`.

## 2. macaca-skill consumer facade
- [ ] 2.1 Add `SkillSnapshotRequest` and builder.
- [ ] 2.2 Add `SkillRuntimeFacade`.
- [ ] 2.3 Add `ExecutableSkillToolSet`.
- [ ] 2.4 Add tests proving facade behavior matches old direct APIs.

## 3. macaca-web migration
- [ ] 3.1 Migrate server startup skill catalog/tool loading.
- [ ] 3.2 Migrate framework runner snapshot construction.
- [ ] 3.3 Migrate skill MCP snapshot construction.
- [ ] 3.4 Migrate app skills status route.

## 4. macaca-app and integration tests
- [ ] 4.1 Migrate `SkillLoader` source inventory to `SkillSourceSet`.
- [ ] 4.2 Migrate integration tests away from deprecated executable skill APIs.

## 5. Verification
- [ ] 5.1 Run `cargo test -p macaca-skill -- --nocapture`.
- [ ] 5.2 Run `cargo test -p macaca-integration-tests fullstack_autodev -- --nocapture`.
- [ ] 5.3 Run `cargo check -p macaca-skill -p macaca-app -p macaca-web -p macaca-runtime-host -p macaca-integration-tests`.
- [ ] 5.4 Run deprecated containment grep and verify no upper non-test crate usage remains.
- [ ] 5.5 Run GitNexus detect changes before commit.
```

- [ ] **Step 6: Create migration delta spec**

Create `openspec/changes/migrate-skill-consumers-to-pattern-primitives/specs/macaca-skill-consumers/spec.md`:

```markdown
## ADDED Requirements

### Requirement: Consumers use skill snapshot request facade
Upper consumers SHALL construct skill snapshots through a request/facade API instead of directly assembling `SkillRuntimeOptions`.

#### Scenario: Framework agent receives same skill catalog
- **GIVEN** an application agent has the same workspace, app directory, and skill policy as before
- **WHEN** `macaca-web` builds the traced framework agent
- **THEN** the resulting skill snapshot prompt, visible skill list, and filtered skill list match the previous direct `SkillRuntime` behavior

### Requirement: Executable skill tools use adapter facade
Upper consumers SHALL expose YAML executable skills through the adapter/facade path instead of deprecated registry instantiation APIs.

#### Scenario: Startup executable skill tools remain available
- **GIVEN** an application skills directory contains executable YAML skills
- **WHEN** `macaca-web` starts and builds the composite tool set
- **THEN** those skill tools are available with the same names, descriptions, schemas, and execution output shape as before

### Requirement: Skill source inventory uses canonical source set
Application-level skill source inventory SHALL use `SkillSourceSet` or an equivalent facade rather than duplicating source precedence logic.

#### Scenario: App skill source precedence remains stable
- **GIVEN** duplicate skill names exist across workspace, app, user, and central sources
- **WHEN** a consumer lists or resolves skill sources
- **THEN** source ordering follows the documented `SkillSourceScope` precedence

### Requirement: Deprecated skill APIs are contained
Upper non-test crates SHALL NOT call deprecated direct skill APIs after migration.

#### Scenario: Deprecated grep only finds compatibility sites
- **WHEN** the repository is scanned for deprecated skill APIs
- **THEN** matches are limited to `macaca-skill` compatibility tests and explicitly documented compatibility wrappers
```

- [ ] **Step 7: Validate migration OpenSpec**

Run:

```bash
openspec validate migrate-skill-consumers-to-pattern-primitives --strict
```

Expected:

```text
Change 'migrate-skill-consumers-to-pattern-primitives' is valid
```

## Task 2: Add `macaca-skill` consumer-facing facade

**Files:**
- Create: `macaca/crates/macaca-skill/src/request.rs`
- Create: `macaca/crates/macaca-skill/src/facade.rs`
- Modify: `macaca/crates/macaca-skill/src/lib.rs`
- Modify: `macaca/crates/macaca-skill/src/registry.rs`
- Test: `macaca/crates/macaca-skill/src/facade.rs`

- [ ] **Step 1: Run impact analysis**

Use GitNexus:

```text
impact(target="SkillRuntime", direction="upstream", repo="agent")
impact(target="SkillRegistry", direction="upstream", repo="agent")
impact(target="SkillTool", direction="upstream", repo="agent")
```

Expected:

```text
Risk for snapshot paths may be HIGH/CRITICAL because web and skill_mcp consume snapshots. Proceed only with additive APIs and no behavior changes.
```

- [ ] **Step 2: Add `SkillSnapshotRequest` builder**

Create `macaca/crates/macaca-skill/src/request.rs`:

```rust
//! Consumer-facing skill snapshot request builder.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::runtime::{SkillPolicy, SkillRuntimeLimits, SkillRuntimeOptions};

#[derive(Debug, Clone)]
pub struct SkillSnapshotRequest {
    pub agent: String,
    pub options: SkillRuntimeOptions,
}

#[derive(Debug, Clone)]
pub struct SkillSnapshotRequestBuilder {
    agent: String,
    workspace_dir: Option<PathBuf>,
    app_dir: Option<PathBuf>,
    bundled_dir: Option<PathBuf>,
    extra_dirs: Vec<PathBuf>,
    policy: SkillPolicy,
    config_flags: HashSet<String>,
    env_overrides: HashSet<String>,
    limits: SkillRuntimeLimits,
}

impl SkillSnapshotRequest {
    pub fn builder(agent: impl Into<String>) -> SkillSnapshotRequestBuilder {
        SkillSnapshotRequestBuilder {
            agent: agent.into(),
            workspace_dir: None,
            app_dir: None,
            bundled_dir: None,
            extra_dirs: Vec::new(),
            policy: SkillPolicy::default(),
            config_flags: HashSet::new(),
            env_overrides: HashSet::new(),
            limits: SkillRuntimeLimits::default(),
        }
    }
}

impl SkillSnapshotRequestBuilder {
    pub fn workspace_dir(mut self, dir: Option<PathBuf>) -> Self {
        self.workspace_dir = dir;
        self
    }

    pub fn app_dir(mut self, dir: Option<PathBuf>) -> Self {
        self.app_dir = dir;
        self
    }

    pub fn bundled_dir(mut self, dir: Option<PathBuf>) -> Self {
        self.bundled_dir = dir;
        self
    }

    pub fn extra_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.extra_dirs = dirs;
        self
    }

    pub fn policy(mut self, policy: SkillPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn config_flags(mut self, flags: HashSet<String>) -> Self {
        self.config_flags = flags;
        self
    }

    pub fn env_overrides(mut self, env: HashSet<String>) -> Self {
        self.env_overrides = env;
        self
    }

    pub fn limits(mut self, limits: SkillRuntimeLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn build(self) -> SkillSnapshotRequest {
        SkillSnapshotRequest {
            agent: self.agent,
            options: SkillRuntimeOptions {
                workspace_dir: self.workspace_dir,
                app_dir: self.app_dir,
                bundled_dir: self.bundled_dir,
                extra_dirs: self.extra_dirs,
                policy: self.policy,
                config_flags: self.config_flags,
                env_overrides: self.env_overrides,
                limits: self.limits,
            },
        }
    }
}
```

- [ ] **Step 3: Add `SkillRuntimeFacade` and executable tool facade**

Create `macaca/crates/macaca-skill/src/facade.rs`:

```rust
//! Consumer-facing skill runtime facades.

use std::path::{Path, PathBuf};

use macaca_proto::{MacacaError, MacacaResult};
use macaca_tools::Tool;

use crate::adapter::SkillToolAdapter;
use crate::catalog::SkillCatalog;
use crate::definition::SkillDefinition;
use crate::registry::SkillRegistry;
use crate::request::SkillSnapshotRequest;
use crate::runtime::{SkillRuntime, SkillSnapshot};
use crate::snapshot::SkillRegistrySnapshot;
use crate::source::SkillSourceSet;
use crate::tool::SkillTool;

#[derive(Debug, Clone, Default)]
pub struct SkillRuntimeFacade {
    runtime: SkillRuntime,
}

impl SkillRuntimeFacade {
    pub fn new() -> Self {
        Self {
            runtime: SkillRuntime,
        }
    }

    pub async fn build_snapshot(
        &self,
        request: SkillSnapshotRequest,
    ) -> MacacaResult<SkillSnapshot> {
        self.runtime
            .build_snapshot(request.agent, request.options)
            .await
    }
}

#[derive(Debug, Clone, Default)]
pub struct ExecutableSkillToolSet {
    registry: SkillRegistry,
}

impl ExecutableSkillToolSet {
    pub fn new() -> Self {
        Self {
            registry: SkillRegistry::new(),
        }
    }

    pub async fn load_from_directory(&mut self, dir: impl AsRef<Path>) -> MacacaResult<usize> {
        let defs = load_executable_skill_definitions(dir.as_ref()).await?;
        let count = defs.len();
        for definition in defs {
            self.registry.register(definition);
        }
        Ok(count)
    }

    pub fn snapshot(&self) -> SkillRegistrySnapshot {
        self.registry.snapshot()
    }

    pub fn into_tools(self) -> Vec<Box<dyn Tool>> {
        self.registry
            .snapshot()
            .skills
            .into_iter()
            .map(|definition| {
                Box::new(SkillTool::from_adapter(SkillToolAdapter::local(definition)))
                    as Box<dyn Tool>
            })
            .collect()
    }

    pub fn tool(&self, name: &str) -> MacacaResult<SkillTool> {
        let definition = self
            .registry
            .get(name)
            .ok_or_else(|| MacacaError::NotFound(format!("Skill '{name}' not found")))?;
        Ok(SkillTool::from_adapter(SkillToolAdapter::local(
            definition.clone(),
        )))
    }
}

#[derive(Debug, Clone)]
pub struct SkillCatalogSourceView {
    sources: SkillSourceSet,
}

impl SkillCatalogSourceView {
    pub fn new(sources: SkillSourceSet) -> Self {
        Self { sources }
    }

    pub fn directories(&self) -> Vec<PathBuf> {
        self.sources.iter().map(|source| source.root.clone()).collect()
    }

    pub async fn load_catalog(&self) -> MacacaResult<SkillCatalog> {
        let mut catalog = SkillCatalog::new();
        for source in self.sources.iter() {
            if source.root.exists() {
                catalog.load_from_directory(&source.root).await?;
            }
        }
        Ok(catalog)
    }
}

pub async fn load_executable_skill_definitions(dir: &Path) -> MacacaResult<Vec<SkillDefinition>> {
    if !dir.exists() {
        return Err(MacacaError::NotFound(format!(
            "Skills directory not found: {}",
            dir.display()
        )));
    }

    let mut definitions = Vec::new();
    let mut entries = tokio::fs::read_dir(dir).await.map_err(MacacaError::Io)?;
    while let Some(entry) = entries.next_entry().await.map_err(MacacaError::Io)? {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str());
        if ext != Some("yaml") && ext != Some("yml") {
            continue;
        }
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => match serde_yaml::from_str::<SkillDefinition>(&content) {
                Ok(definition) => definitions.push(definition),
                Err(e) => tracing::warn!("failed to parse skill {:?}: {}", path, e),
            },
            Err(e) => tracing::warn!("failed to read {:?}: {}", path, e),
        }
    }
    definitions.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(definitions)
}
```

- [ ] **Step 4: Export facade/request**

Modify `macaca/crates/macaca-skill/src/lib.rs`:

```rust
pub mod facade;
pub mod request;

pub use facade::{
    load_executable_skill_definitions, ExecutableSkillToolSet, SkillCatalogSourceView,
    SkillRuntimeFacade,
};
pub use request::{SkillSnapshotRequest, SkillSnapshotRequestBuilder};
```

- [ ] **Step 5: Add facade tests**

Add tests in `facade.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use macaca_tools::{Tool, ToolCommand, ToolCommandExecutor};

    #[tokio::test]
    async fn executable_toolset_loads_yaml_and_exposes_tools() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("echo.yaml"),
            r#"
name: echo-skill
description: Echoes input
entry_point:
  type: shell
  command: echo
  args: ["hello"]
"#,
        )
        .await
        .unwrap();

        let mut toolset = ExecutableSkillToolSet::new();
        let loaded = toolset.load_from_directory(dir.path()).await.unwrap();
        assert_eq!(loaded, 1);

        let tool = toolset.tool("echo-skill").unwrap();
        assert_eq!(tool.name(), "echo-skill");
        let result = ToolCommandExecutor::execute_command(&tool, ToolCommand::new(serde_json::json!({})))
            .await
            .unwrap();
        assert_eq!(result["exit_code"], 0);
        assert!(result["stdout"].as_str().unwrap().contains("hello"));
    }

    #[tokio::test]
    async fn runtime_facade_builds_snapshot_from_request() {
        let app = tempfile::tempdir().unwrap();
        let skill_dir = app.path().join("skills").join("writer");
        tokio::fs::create_dir_all(&skill_dir).await.unwrap();
        tokio::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: writer\ndescription: Writing\n---\nBody",
        )
        .await
        .unwrap();

        let request = SkillSnapshotRequest::builder("agent")
            .app_dir(Some(app.path().to_path_buf()))
            .build();
        let snapshot = SkillRuntimeFacade::new().build_snapshot(request).await.unwrap();
        assert_eq!(snapshot.skills.len(), 1);
        assert_eq!(snapshot.skills[0].name, "writer");
    }
}
```

- [ ] **Step 6: Run skill tests**

Run:

```bash
cargo test -p macaca-skill -- --nocapture
```

Expected:

```text
test result: ok.
```

## Task 3: Migrate `macaca-web` startup skill loading

**Files:**
- Modify: `macaca/crates/macaca-web/src/lib.rs`

- [ ] **Step 1: Run impact analysis**

Use GitNexus:

```text
impact(target="start_server", direction="upstream", repo="agent")
```

Expected:

```text
Risk may be HIGH/CRITICAL because server startup builds tools and state. Proceed with exact behavior preservation.
```

- [ ] **Step 2: Replace executable registry startup path**

Modify imports:

```rust
use macaca_skill::{ExecutableSkillToolSet, SkillCatalog};
```

Replace:

```rust
let mut skill_registry = SkillRegistry::new();
match skill_registry.load_from_directory(dir).await {
    Ok(n) => {
        let skill_tools = skill_registry.instantiate_all_tools();
        info!(count = n, "Executable skill tools loaded");
        all_tools.extend(skill_tools);
    }
    Err(e) => tracing::warn!("Failed to load executable skills: {e}"),
}
```

With:

```rust
let mut skill_tools = ExecutableSkillToolSet::new();
match skill_tools.load_from_directory(dir).await {
    Ok(n) => {
        let loaded_tools = skill_tools.into_tools();
        info!(count = n, "Executable skill tools loaded");
        all_tools.extend(loaded_tools);
    }
    Err(e) => tracing::warn!("Failed to load executable skills: {e}"),
}
```

- [ ] **Step 3: Keep knowledge catalog behavior unchanged**

Do not change `SkillCatalog::load_from_directory` in this task unless `SkillCatalogSourceView` has already proven equivalent. `SkillCatalog` direct loading is not deprecated and is still valid.

- [ ] **Step 4: Verify web compile**

Run:

```bash
cargo check -p macaca-web
```

Expected:

```text
Finished `dev` profile
```

## Task 4: Migrate snapshot construction in web

**Files:**
- Modify: `macaca/crates/macaca-web/src/framework_runner.rs`
- Modify: `macaca/crates/macaca-web/src/skill_mcp.rs`
- Modify: `macaca/crates/macaca-web/src/routes.rs`

- [ ] **Step 1: Run impact analysis**

Use GitNexus:

```text
impact(target="build_system_prompt", direction="upstream", repo="agent")
impact(target="load_or_build_skill_snapshot", direction="upstream", repo="agent")
impact(target="get_app_skills", direction="upstream", repo="agent")
```

Expected:

```text
These paths affect prompt construction, skill-backed MCP, and status API.
```

- [ ] **Step 2: Migrate framework runner request construction**

In `framework_runner.rs`, import:

```rust
use macaca_skill::{SkillRuntimeFacade, SkillSnapshotRequest};
```

Replace direct `SkillRuntime.build_snapshot(...)` call with:

```rust
let request = SkillSnapshotRequest::builder(agent_name)
    .workspace_dir(workspace_root)
    .app_dir(app_dir)
    .policy(skill_policy)
    .build();
let snapshot = SkillRuntimeFacade::new().build_snapshot(request).await;
```

Do not change event log payloads for `skill_catalog_built` and `skill_snapshot_created`.

- [ ] **Step 3: Migrate skill MCP snapshot construction**

In `skill_mcp.rs`, replace direct snapshot construction with:

```rust
let request = SkillSnapshotRequest::builder(agent_name)
    .workspace_dir(workspace_dir)
    .app_dir(Some(app.path))
    .policy(policy)
    .build();
let snapshot = SkillRuntimeFacade::new().build_snapshot(request).await.ok()?;
```

Keep `definitions_from_skill_snapshot(&snapshot)` unchanged.

- [ ] **Step 4: Migrate skill status route**

In `routes.rs`, replace direct snapshot construction with:

```rust
let request = SkillSnapshotRequest::builder(agent.name.clone())
    .workspace_dir(workspace_root.clone())
    .app_dir(Some(app.path.clone()))
    .policy(policy)
    .build();
let snapshot = SkillRuntimeFacade::new()
    .build_snapshot(request)
    .await
    .map_err(|e| proto_err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
```

- [ ] **Step 5: Verify no behavior-specific payload changed**

Run:

```bash
rg -n "skill_catalog_built|skill_snapshot_created|definitions_from_skill_snapshot|probe_skill_mcp_servers" macaca/crates/macaca-web/src
```

Expected:

```text
Existing event names and MCP conversion calls remain present.
```

## Task 5: Migrate `macaca-app::SkillLoader` source inventory

**Files:**
- Modify: `macaca/crates/macaca-app/src/skills.rs`

- [ ] **Step 1: Run impact analysis**

Use GitNexus:

```text
impact(target="SkillLoader", direction="upstream", repo="agent")
```

Expected:

```text
Main impact should be macaca-app tests and any web/app consumers.
```

- [ ] **Step 2: Use `SkillSourceSet` for source directory listing**

Add dependency in `macaca/crates/macaca-app/Cargo.toml` if not already present:

```toml
macaca-skill = { workspace = true }
```

Modify `SkillLoader::list_skill_dirs` to build a `SkillRuntimeOptions` and `SkillSourceSet`:

```rust
let sources = macaca_skill::SkillSourceSet::from_options(&macaca_skill::SkillRuntimeOptions {
    app_dir: self.app_dir.clone().and_then(|dir| dir.parent().map(Path::to_path_buf)),
    extra_dirs: vec![self.global_dir.clone()],
    ..Default::default()
});
sources
    .iter()
    .map(|source| source.root.clone())
    .filter(|dir| dir.exists())
    .collect()
```

If this changes the exact app/global semantics, use a smaller helper that only centralizes `SkillSource` construction while preserving old app-dir-is-skills-dir behavior:

```rust
let mut dirs = Vec::new();
for dir in [&self.global_dir, self.app_dir.as_ref().unwrap_or(&self.global_dir)] {
    if dir.exists() && !dirs.contains(dir) {
        dirs.push(dir.clone());
    }
}
dirs
```

Preferred implementation must preserve current tests exactly.

- [ ] **Step 3: Keep `skill_exists`, `get_skill_path`, and `list_skill_names` behavior**

Refactor only if tests prove exact behavior. Do not change app skill priority over global skill.

- [ ] **Step 4: Verify app tests**

Run:

```bash
cargo test -p macaca-app skills::tests -- --nocapture
```

Expected:

```text
test result: ok.
```

## Task 6: Migrate integration tests away from deprecated APIs

**Files:**
- Modify: `macaca/crates/macaca-integration-tests/tests/fullstack_autodev.rs`
- Modify: `macaca/crates/macaca-integration-tests/tests/live_fullstack_autodev.rs`

- [ ] **Step 1: Replace executable skill registry tests**

In `fullstack_autodev.rs`, replace:

```rust
let mut registry = SkillRegistry::new();
let loaded = registry.load_from_directory(&skills_dir).await.unwrap();
```

With:

```rust
let mut toolset = macaca_skill::ExecutableSkillToolSet::new();
let loaded = toolset.load_from_directory(&skills_dir).await.unwrap();
let snapshot = toolset.snapshot();
```

Replace `registry.get("openspec")` assertions with:

```rust
assert!(snapshot.skills.iter().any(|skill| skill.name == "openspec"));
```

- [ ] **Step 2: Replace executable tool instantiation test**

Replace:

```rust
let openspec_tool = registry.instantiate_tool("openspec").unwrap();
```

With:

```rust
let openspec_tool = toolset.tool("openspec").unwrap();
```

- [ ] **Step 3: Keep `SkillCatalog` tests unchanged unless deprecated**

`SkillCatalog::load_from_directory` is not deprecated. Keep knowledge skill tests unchanged unless a new catalog facade is explicitly introduced.

- [ ] **Step 4: Verify integration test compile/run**

Run:

```bash
cargo test -p macaca-integration-tests fullstack_autodev -- --nocapture
```

Expected:

```text
test result: ok.
```

## Task 7: Verification, grep containment, and commit

**Files:**
- Modify: `openspec/changes/migrate-skill-consumers-to-pattern-primitives/tasks.md`

- [ ] **Step 1: Run full targeted checks**

Run:

```bash
cargo test -p macaca-skill -- --nocapture
cargo test -p macaca-app skills::tests -- --nocapture
cargo test -p macaca-integration-tests fullstack_autodev -- --nocapture
cargo check -p macaca-skill -p macaca-app -p macaca-web -p macaca-runtime-host -p macaca-integration-tests
openspec validate refactor-macaca-skill-patterns --strict
openspec validate migrate-skill-consumers-to-pattern-primitives --strict
```

Expected:

```text
All tests/checks pass.
```

- [ ] **Step 2: Run deprecated containment grep**

Run:

```bash
rg -n "SkillRegistry::new\\(|load_from_directory\\(|instantiate_all_tools\\(|instantiate_tool\\(|SkillTool::new\\(" macaca/crates --glob '*.rs'
```

Expected:

```text
No upper non-test crate calls deprecated SkillRegistry/SkillTool APIs.
Remaining matches are either:
- macaca-skill compatibility wrappers/tests
- unrelated APIs such as AgentPersona::load_from_directory
- non-deprecated SkillCatalog::load_from_directory
```

- [ ] **Step 3: Run GitNexus detect changes**

Use GitNexus:

```text
detect_changes(scope="all", repo="agent")
```

Expected:

```text
Affected flows include expected skill startup, skill snapshot, skill-backed MCP, app skill loader, and integration test paths only.
```

- [ ] **Step 4: Mark OpenSpec tasks complete**

Update:

```text
openspec/changes/migrate-skill-consumers-to-pattern-primitives/tasks.md
```

Set completed items to `- [x]` only after verification has actually passed.

- [ ] **Step 5: Commit**

Run:

```bash
git add \
  docs/superpowers/plans/2026-05-02-migrate-macaca-skill-consumers.md \
  openspec/changes/refactor-macaca-skill-patterns \
  openspec/changes/migrate-skill-consumers-to-pattern-primitives \
  macaca/crates/macaca-skill \
  macaca/crates/macaca-web \
  macaca/crates/macaca-app \
  macaca/crates/macaca-integration-tests
git commit -m "refactor: migrate skill consumers to pattern primitives"
gitnexus analyze
```

Expected:

```text
Commit succeeds and GitNexus index rebuilds.
```

## Risks

- `framework_runner.rs` snapshot construction affects every traced agent prompt. Mitigation: use request builder that maps 1:1 to existing `SkillRuntimeOptions`, keep event payloads unchanged, run web compile and skill runtime tests.
- `skill_mcp.rs` snapshot construction affects MCP tool registration. Mitigation: leave `definitions_from_skill_snapshot` and MCP runtime path unchanged.
- `macaca-web/src/lib.rs` startup tool loading affects executable YAML skills. Mitigation: `ExecutableSkillToolSet` must preserve same output `Tool` behavior and JSON result shape.
- `macaca-app::SkillLoader` may have slightly different semantics from `SkillSourceSet`. Mitigation: preserve current tests first; if exact mapping is not clean, keep app loader behavior and only introduce adapter helpers in this round.
- There is existing dirty state from the previous `macaca-skill` refactor plus generated Playwright/page snapshot artifacts. Mitigation: only stage code/docs/OpenSpec files relevant to this migration; leave runtime artifacts untracked.

## Handoff

Plan complete. The next step is to create the root OpenSpec migration proposal `migrate-skill-consumers-to-pattern-primitives`; implementation should start only after approval.
