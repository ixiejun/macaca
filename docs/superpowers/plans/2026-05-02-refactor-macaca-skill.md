# macaca-skill Design-Pattern Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 对 `macaca-skill` 做渐进式、additive-first、行为 1:1 保持的设计模式重构，为后续所有 application 的 skill discovery、metadata gating、snapshot、tool exposure、runtime lifecycle 提供稳定基础抽象。

**Architecture:** 本轮只重构 `macaca-skill` crate 内部能力边界，新增 policy chain、source factory、snapshot/reload、tool adapter、runtime handle 等入口，旧 API 标记 deprecated 但不删除。上层 `macaca-web` / `macaca-runtime-host` 继续可用，后续单独提案迁移消费方。

**Tech Stack:** Rust, Tokio, serde/serde_yaml, macaca-proto, macaca-tools, OpenSpec, cargo test/check.

---

## Context

`macaca-skill` 当前承担两类 skill：

- Knowledge skill：标准 `SKILL.md`，用于 prompt catalog、progressive disclosure、metadata gating、snapshot。
- Executable skill：`*.yaml` / `*.yml`，通过 `SkillRegistry` 变成 `SkillTool`，最终进入工具系统。

当前主要问题：

- `runtime.rs` 同时做 source discovery、metadata gating、prompt formatting、snapshot 构建，职责过重。
- `filter_reason()` 是集中式 if/else gating，不利于扩展更多 metadata gate。
- `SkillRegistry` 只有内存 map，没有 snapshot/reload contract。
- `SkillTool` 直接执行 shell/script/MCP 分支，缺少 tool adapter / runtime proxy 边界。
- `SkillProvisioner` 只返回数量，不能表达 installed/provisioned/active/error/released 这类 lifecycle。
- 上层消费方已经有 skill snapshot 和 skill_mcp 路径，但底层 skill runtime 还没有完整基础设施边界。

## Superpowers Brainstorm

### Option A: Minimal helper extraction

只从 `runtime.rs` 抽出几个私有 helper 文件，行为完全不动。

Trade-offs:

- 优点：风险最低，改动很小。
- 缺点：不能解决 metadata gating、snapshot/reload、runtime handle、tool adapter 这些正式边界问题。
- 结论：不推荐。它只是整理文件，不能支撑 7x24 Agent OS 的标准 skill runtime。

### Option B: Additive-first primitives inside `macaca-skill`

在 `macaca-skill` 内新增清晰的 primitives：`SkillExposurePolicy` / `SkillPolicyChain`、`SkillSourceFactory`、`SkillRegistrySnapshot`、`SkillToolAdapter`、`SkillRuntimeHandle`。旧 API 全部保留并标记 deprecated，内部逐步委托到新抽象。

Trade-offs:

- 优点：小步、可测试、可回滚；不破坏 web/runtime-host；符合阶段 2-4 的重构顺序。
- 缺点：短期存在新旧 API 并存，需要后续单独做消费方迁移。
- 结论：推荐。它在不破坏现有功能的前提下建立真正的 skill runtime contract。

### Option C: 一次性把 skill + MCP + framework toolkit 合并为统一 runtime

直接把 skill-backed MCP、OS MCP registry、framework toolkit 全部迁移到一个统一 runtime。

Trade-offs:

- 优点：最终形态最干净。
- 缺点：跨 `macaca-skill`、`macaca-runtime-host`、`macaca-framework`、`macaca-web`，风险高，容易再次引入实时 trace / resource cleanup 回归。
- 结论：本轮不采用。应在 `macaca-skill` contract 稳定后单独做上层迁移。

## Recommended Design

采用 Option B。

本轮重构只做 additive-first primitives，不改上层行为：

- `runtime.rs` 的旧 `SkillRuntime::build_snapshot()` 保持可用。
- `SkillRegistry::{load_from_directory, instantiate_all_tools}` 保持可用但标记 deprecated。
- `SkillTool::new()` 保持可用但标记 deprecated。
- 新入口使用 Strategy / Chain of Responsibility / Abstract Factory / Registry / Adapter / Proxy / State / Memento。
- 所有新增类型必须不硬编码 application name、workflow、driver name。
- 所有 contract tests 只使用通用 fixture，不能针对 `FULLSTACK-AUTODEV` 或 `NEWSROOM-AUTOWRITER`。

## Design Pattern Mapping

- `Strategy + Chain of Responsibility`: `SkillExposurePolicy` 与 `SkillPolicyChain` 替代集中式 gating if/else。
- `Abstract Factory + Registry`: `SkillSourceFactory` / `SkillSourceSet` 统一 workspace/app/user/bundled/extra source 创建。
- `Memento`: `SkillRegistrySnapshot` / `SkillSnapshotMemento` 支持 registry 和 runtime snapshot 持久化。
- `Adapter + Proxy`: `SkillToolAdapter` / `SkillRuntimeProxy` 隔离 shell/script/MCP/local runtime 差异。
- `State`: `SkillRuntimeState` / `SkillRuntimeHandle` 表达 installed/provisioned/active/error/released。
- `Facade`: 后续可在本 crate 内新增 `SkillRuntimeFacade`，但本轮只在最后切片评估是否需要，避免过度设计。

## Files

- Modify: `macaca/crates/macaca-skill/src/lib.rs`
- Modify: `macaca/crates/macaca-skill/src/runtime.rs`
- Modify: `macaca/crates/macaca-skill/src/registry.rs`
- Modify: `macaca/crates/macaca-skill/src/tool.rs`
- Modify: `macaca/crates/macaca-skill/src/provisioner.rs`
- Create: `macaca/crates/macaca-skill/src/policy.rs`
- Create: `macaca/crates/macaca-skill/src/source.rs`
- Create: `macaca/crates/macaca-skill/src/snapshot.rs`
- Create: `macaca/crates/macaca-skill/src/adapter.rs`
- Create: `macaca/crates/macaca-skill/src/handle.rs`
- Create: `macaca/openspec/changes/refactor-macaca-skill-patterns/proposal.md`
- Create: `macaca/openspec/changes/refactor-macaca-skill-patterns/design.md`
- Create: `macaca/openspec/changes/refactor-macaca-skill-patterns/tasks.md`
- Create: `macaca/openspec/changes/refactor-macaca-skill-patterns/specs/macaca-skill-core/spec.md`

## Task 1: OpenSpec proposal and contract

**Files:**
- Create: `macaca/openspec/changes/refactor-macaca-skill-patterns/proposal.md`
- Create: `macaca/openspec/changes/refactor-macaca-skill-patterns/design.md`
- Create: `macaca/openspec/changes/refactor-macaca-skill-patterns/tasks.md`
- Create: `macaca/openspec/changes/refactor-macaca-skill-patterns/specs/macaca-skill-core/spec.md`

- [ ] **Step 1: Review OpenSpec baseline**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
openspec list
openspec list --specs
```

Expected:

```text
Changes:
  migrate-goal-pipeline-to-framework     No tasks
No specs found.
```

- [ ] **Step 2: Create proposal**

Create `macaca/openspec/changes/refactor-macaca-skill-patterns/proposal.md`:

```markdown
# Change: Refactor macaca-skill with design-pattern primitives

## Why
`macaca-skill` is the Agent OS foundation for standard skills, metadata gating, snapshot recovery, executable skill exposure, and skill lifecycle. Current implementations mix discovery, filtering, prompt formatting, tool wrapping, and provisioning in a few concrete modules, which makes future MCP/skill/runtime expansion fragile.

## What Changes
- Add additive-first skill exposure policy chain primitives.
- Add source factory primitives for skill discovery sources.
- Add registry snapshot/reload primitives for executable skills.
- Add skill tool adapter/proxy primitives for executable skill tool exposure.
- Add runtime handle/state primitives for provisioning lifecycle.
- Mark legacy direct APIs as deprecated but keep them callable for migration.

## Impact
- Affected specs: macaca-skill-core
- Affected code: `crates/macaca-skill/src/*`
- No application-specific logic, workflow-specific logic, or driver-name hardcoding is introduced.
```

- [ ] **Step 3: Create design**

Create `macaca/openspec/changes/refactor-macaca-skill-patterns/design.md`:

```markdown
## Context

`macaca-skill` supports two skill families:

- Knowledge skills from `SKILL.md`, exposed through prompt catalog and snapshots.
- Executable skills from YAML, exposed as `macaca_tools::Tool`.

This change keeps existing behavior but introduces stable primitives so upper crates can migrate without directly depending on concrete loader/filter/tool branches.

## Goals

- Preserve behavior 1:1 for existing `SkillRuntime`, `SkillRegistry`, `SkillCatalog`, `SkillProvisioner`, and `SkillTool` callers.
- Add Strategy/Chain primitives for metadata gating.
- Add Factory/Registry primitives for discovery sources.
- Add Memento primitives for executable registry snapshots.
- Add Adapter/Proxy primitives for executable tool exposure.
- Add State primitives for skill provisioning/runtime handle status.

## Non-Goals

- Do not migrate `macaca-web` or `macaca-runtime-host` consumers in this change.
- Do not implement marketplace install/search/update.
- Do not move MCP lifecycle ownership into `macaca-skill`; MCP remains Agent OS runtime responsibility.

## Decisions

- Legacy APIs remain available and are marked `#[deprecated]` only after new additive APIs exist.
- Policy evaluation returns stable reason strings already used by status APIs: `denied_by_policy`, `disabled_model_invocation`, `os_mismatch`, `missing_bin`, `missing_env`, `missing_config`.
- Runtime handles model lifecycle state but do not spawn long-lived MCP/browser resources.
```

- [ ] **Step 4: Create tasks checklist**

Create `macaca/openspec/changes/refactor-macaca-skill-patterns/tasks.md`:

```markdown
## 1. OpenSpec
- [ ] 1.1 Add proposal, design, tasks, and delta spec.
- [ ] 1.2 Validate with `openspec validate refactor-macaca-skill-patterns --strict`.

## 2. Policy Chain
- [ ] 2.1 Add `policy.rs`.
- [ ] 2.2 Route runtime filtering through `SkillPolicyChain`.
- [ ] 2.3 Add policy tests.

## 3. Source Factory
- [ ] 3.1 Add `source.rs`.
- [ ] 3.2 Route runtime source construction through `SkillSourceSet`.
- [ ] 3.3 Add precedence tests.

## 4. Snapshot / Reload
- [ ] 4.1 Add `snapshot.rs`.
- [ ] 4.2 Add `SkillRegistry::snapshot` and `SkillRegistry::reload_from_snapshot`.
- [ ] 4.3 Mark direct load APIs deprecated only after replacements exist.

## 5. Tool Adapter / Runtime Handle
- [ ] 5.1 Add `adapter.rs`.
- [ ] 5.2 Add `handle.rs`.
- [ ] 5.3 Route `SkillTool` through adapter/proxy.
- [ ] 5.4 Extend provisioner to return runtime handles additively.

## 6. Verification
- [ ] 6.1 Run `cargo test -p macaca-skill -- --nocapture`.
- [ ] 6.2 Run `cargo check -p macaca-skill -p macaca-web -p macaca-runtime-host`.
- [ ] 6.3 Run deprecated API containment grep.
- [ ] 6.4 Run GitNexus detect changes before commit.
```

- [ ] **Step 5: Create delta spec**

Create `macaca/openspec/changes/refactor-macaca-skill-patterns/specs/macaca-skill-core/spec.md`:

```markdown
## ADDED Requirements

### Requirement: Additive skill exposure policy chain
The system SHALL provide a skill exposure policy chain that evaluates existing skill metadata gates without changing visible/filter behavior.

#### Scenario: Existing metadata gates are preserved
- **GIVEN** a skill requires a missing env var
- **WHEN** a snapshot is built through the refactored runtime
- **THEN** the skill is filtered with reason `missing_env`

### Requirement: Additive skill source factory
The system SHALL provide source factory primitives that produce workspace, application, user, bundled, and extra skill sources in the documented precedence order.

#### Scenario: Workspace source wins by precedence
- **GIVEN** duplicate skill names in workspace and application sources
- **WHEN** a snapshot is built
- **THEN** the workspace skill is selected

### Requirement: Executable skill registry snapshot
The system SHALL provide registry snapshot and reload primitives for executable skill definitions.

#### Scenario: Registry reload preserves executable skills
- **GIVEN** a registry containing two executable skill definitions
- **WHEN** the registry is snapshotted and reloaded into a new registry
- **THEN** both skill definitions are available by name

### Requirement: Skill tool adapter
The system SHALL provide an adapter/proxy boundary for executable skill tool calls while preserving existing shell/script/MCP behavior.

#### Scenario: Shell skill still executes through tool command executor
- **GIVEN** a shell executable skill
- **WHEN** it is exposed as a tool through the adapter
- **THEN** the tool command executor returns stdout, stderr, exit_code, and command fields as before

### Requirement: Skill runtime lifecycle handle
The system SHALL provide a runtime handle that represents skill lifecycle state without taking MCP lifecycle ownership away from the Agent OS MCP runtime.

#### Scenario: Provisioned skill handle records target client
- **GIVEN** a skill is provisioned to a client
- **WHEN** the additive handle API is used
- **THEN** the returned handle records the skill id, client id, target path, and `Provisioned` state
```

- [ ] **Step 6: Validate proposal**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
openspec validate refactor-macaca-skill-patterns --strict
```

Expected:

```text
Change 'refactor-macaca-skill-patterns' is valid
```

## Task 2: Policy chain extraction

**Files:**
- Create: `macaca/crates/macaca-skill/src/policy.rs`
- Modify: `macaca/crates/macaca-skill/src/runtime.rs`
- Modify: `macaca/crates/macaca-skill/src/lib.rs`

- [ ] **Step 1: Run GitNexus impact before editing `SkillRuntime::build_snapshot`**

Run:

```bash
cd /Users/quantum/Code/dev/agent
```

Use GitNexus:

```text
impact(target="build_snapshot", direction="upstream", repo="agent")
impact(target="SkillRuntime", direction="upstream", repo="agent")
```

Expected:

```text
Direct callers include macaca-web framework runner, routes, skill_mcp, and integration tests. Risk must be reviewed before editing.
```

- [ ] **Step 2: Add policy primitives**

Create `macaca/crates/macaca-skill/src/policy.rs`:

```rust
//! Skill exposure policy chain.

use std::collections::HashSet;
use std::env;

use crate::agent_skill::SkillEntry;
use crate::runtime::SkillRuntimeOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    Deny(String),
}

impl PolicyDecision {
    pub fn deny(reason: impl Into<String>) -> Self {
        Self::Deny(reason.into())
    }
}

pub trait SkillExposurePolicy: Send + Sync {
    fn allows(&self, entry: &SkillEntry, ctx: &SkillExposureContext<'_>) -> PolicyDecision;
}

pub struct SkillExposureContext<'a> {
    pub allow: Option<&'a HashSet<String>>,
    pub deny: &'a HashSet<String>,
    pub options: &'a SkillRuntimeOptions,
}

pub struct SkillPolicyChain {
    policies: Vec<Box<dyn SkillExposurePolicy>>,
}

impl SkillPolicyChain {
    pub fn default_chain() -> Self {
        Self {
            policies: vec![
                Box::new(AllowDenyPolicy),
                Box::new(ModelInvocationPolicy),
                Box::new(MetadataAlwaysPolicy),
                Box::new(OsPolicy),
                Box::new(BinaryPolicy),
                Box::new(EnvironmentPolicy),
                Box::new(ConfigPolicy),
            ],
        }
    }

    pub fn evaluate(&self, entry: &SkillEntry, ctx: &SkillExposureContext<'_>) -> PolicyDecision {
        for policy in &self.policies {
            match policy.allows(entry, ctx) {
                PolicyDecision::Allow => {}
                denied @ PolicyDecision::Deny(_) => return denied,
            }
        }
        PolicyDecision::Allow
    }
}

struct AllowDenyPolicy;

impl SkillExposurePolicy for AllowDenyPolicy {
    fn allows(&self, entry: &SkillEntry, ctx: &SkillExposureContext<'_>) -> PolicyDecision {
        let key = entry
            .metadata
            .skill_key
            .as_deref()
            .unwrap_or(entry.skill.name.as_str());
        if ctx.deny.contains(entry.skill.name.as_str()) || ctx.deny.contains(key) {
            return PolicyDecision::deny("denied_by_policy");
        }
        if let Some(allow) = ctx.allow {
            if !allow.contains(entry.skill.name.as_str()) && !allow.contains(key) {
                return PolicyDecision::deny("denied_by_policy");
            }
        }
        PolicyDecision::Allow
    }
}

struct ModelInvocationPolicy;

impl SkillExposurePolicy for ModelInvocationPolicy {
    fn allows(&self, entry: &SkillEntry, _ctx: &SkillExposureContext<'_>) -> PolicyDecision {
        if entry.invocation.disable_model_invocation {
            return PolicyDecision::deny("disabled_model_invocation");
        }
        PolicyDecision::Allow
    }
}

struct MetadataAlwaysPolicy;

impl SkillExposurePolicy for MetadataAlwaysPolicy {
    fn allows(&self, entry: &SkillEntry, _ctx: &SkillExposureContext<'_>) -> PolicyDecision {
        if entry.metadata.always {
            return PolicyDecision::Allow;
        }
        PolicyDecision::Allow
    }
}

struct OsPolicy;

impl SkillExposurePolicy for OsPolicy {
    fn allows(&self, entry: &SkillEntry, _ctx: &SkillExposureContext<'_>) -> PolicyDecision {
        if !entry.metadata.os.is_empty()
            && !entry.metadata.os.iter().any(|os| os_matches_current(os))
        {
            return PolicyDecision::deny("os_mismatch");
        }
        PolicyDecision::Allow
    }
}

struct BinaryPolicy;

impl SkillExposurePolicy for BinaryPolicy {
    fn allows(&self, entry: &SkillEntry, _ctx: &SkillExposureContext<'_>) -> PolicyDecision {
        for bin in &entry.metadata.requires_bins {
            if !has_binary(bin) {
                return PolicyDecision::deny("missing_bin");
            }
        }
        if !entry.metadata.requires_any_bins.is_empty()
            && !entry
                .metadata
                .requires_any_bins
                .iter()
                .any(|bin| has_binary(bin))
        {
            return PolicyDecision::deny("missing_bin");
        }
        PolicyDecision::Allow
    }
}

struct EnvironmentPolicy;

impl SkillExposurePolicy for EnvironmentPolicy {
    fn allows(&self, entry: &SkillEntry, ctx: &SkillExposureContext<'_>) -> PolicyDecision {
        for env_name in &entry.metadata.requires_env {
            if env::var_os(env_name).is_none() && !ctx.options.env_overrides.contains(env_name) {
                return PolicyDecision::deny("missing_env");
            }
        }
        PolicyDecision::Allow
    }
}

struct ConfigPolicy;

impl SkillExposurePolicy for ConfigPolicy {
    fn allows(&self, entry: &SkillEntry, ctx: &SkillExposureContext<'_>) -> PolicyDecision {
        for config in &entry.metadata.requires_config {
            if !ctx.options.config_flags.contains(config) {
                return PolicyDecision::deny("missing_config");
            }
        }
        PolicyDecision::Allow
    }
}

pub fn normalize_policy_set<'a>(items: impl Iterator<Item = &'a str>) -> HashSet<String> {
    items
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn has_binary(bin: &str) -> bool {
    let bin = bin.trim();
    if bin.is_empty() {
        return false;
    }
    let Some(paths) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&paths).any(|dir| dir.join(bin).is_file())
}

fn os_matches_current(skill_os: &str) -> bool {
    let requested = skill_os.trim().to_ascii_lowercase();
    let current = env::consts::OS;
    requested == current
        || matches!(
            (requested.as_str(), current),
            ("darwin", "macos") | ("macos", "macos")
        )
}
```

- [ ] **Step 3: Route `filter_entries` through policy chain**

Modify `runtime.rs` imports:

```rust
use crate::policy::{
    normalize_policy_set, PolicyDecision, SkillExposureContext, SkillPolicyChain,
};
```

Replace `filter_entries` and `filter_reason` implementation with:

```rust
fn filter_entries(
    entries: Vec<SkillEntry>,
    options: &SkillRuntimeOptions,
) -> (Vec<SkillEntry>, Vec<FilteredSkill>) {
    let allow = options
        .policy
        .allow
        .as_ref()
        .map(|items| normalize_policy_set(items.iter().map(String::as_str)));
    let deny = normalize_policy_set(options.policy.deny.iter().map(String::as_str));
    let chain = SkillPolicyChain::default_chain();

    let mut visible = Vec::new();
    let mut filtered = Vec::new();
    for entry in entries {
        let name = entry.skill.name.clone();
        let source = entry.skill.source.clone();
        let ctx = SkillExposureContext {
            allow: allow.as_ref(),
            deny: &deny,
            options,
        };
        match chain.evaluate(&entry, &ctx) {
            PolicyDecision::Allow => visible.push(entry),
            PolicyDecision::Deny(reason) => filtered.push(FilteredSkill {
                name,
                reason,
                source,
            }),
        }
    }
    (visible, filtered)
}
```

Remove the old private `filter_reason`, `normalize_set`, `has_binary`, and `os_matches_current` from `runtime.rs`.

- [ ] **Step 4: Export policy module**

Modify `lib.rs`:

```rust
pub mod policy;

pub use policy::{
    PolicyDecision, SkillExposureContext, SkillExposurePolicy, SkillPolicyChain,
};
```

- [ ] **Step 5: Run tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-skill runtime::tests -- --nocapture
```

Expected:

```text
test result: ok.
```

## Task 3: Source factory extraction

**Files:**
- Create: `macaca/crates/macaca-skill/src/source.rs`
- Modify: `macaca/crates/macaca-skill/src/runtime.rs`
- Modify: `macaca/crates/macaca-skill/src/lib.rs`

- [ ] **Step 1: Run GitNexus impact before editing discovery path**

Use GitNexus:

```text
impact(target="discover_skill_entries", direction="upstream", repo="agent")
```

Expected:

```text
Direct callers include SkillRuntime::build_snapshot only; upstream remains web/framework/status paths.
```

- [ ] **Step 2: Add source primitives**

Create `macaca/crates/macaca-skill/src/source.rs`:

```rust
//! Skill source factory primitives.

use std::env;
use std::path::PathBuf;

use crate::agent_skill::SkillSourceScope;
use crate::runtime::SkillRuntimeOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSource {
    pub root: PathBuf,
    pub scope: SkillSourceScope,
    pub label: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillSourceSet {
    sources: Vec<SkillSource>,
}

impl SkillSourceSet {
    pub fn from_options(options: &SkillRuntimeOptions) -> Self {
        let mut sources = Vec::new();
        if let Some(workspace) = &options.workspace_dir {
            sources.push(SkillSource {
                root: workspace.join("skills"),
                scope: SkillSourceScope::Workspace,
                label: "workspace".into(),
            });
        }
        if let Some(app_dir) = &options.app_dir {
            sources.push(SkillSource {
                root: app_dir.join(".agents").join("skills"),
                scope: SkillSourceScope::ProjectAgents,
                label: "project_agents".into(),
            });
            sources.push(SkillSource {
                root: app_dir.join("skills"),
                scope: SkillSourceScope::Application,
                label: "application".into(),
            });
        }
        if let Some(home) = home_dir() {
            sources.push(SkillSource {
                root: home.join(".agents").join("skills"),
                scope: SkillSourceScope::UserAgents,
                label: "user_agents".into(),
            });
            sources.push(SkillSource {
                root: home.join(".macaca").join("skills"),
                scope: SkillSourceScope::MacacaCentral,
                label: "macaca_central".into(),
            });
        }
        if let Some(bundled) = &options.bundled_dir {
            sources.push(SkillSource {
                root: bundled.clone(),
                scope: SkillSourceScope::Bundled,
                label: "bundled".into(),
            });
        }
        for dir in &options.extra_dirs {
            sources.push(SkillSource {
                root: dir.clone(),
                scope: SkillSourceScope::Extra,
                label: "extra".into(),
            });
        }
        Self { sources }
    }

    pub fn iter(&self) -> impl Iterator<Item = &SkillSource> {
        self.sources.iter()
    }

    pub fn len(&self) -> usize {
        self.sources.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}
```

- [ ] **Step 3: Use `SkillSourceSet` in runtime discovery**

Modify `runtime.rs`:

```rust
use crate::source::SkillSourceSet;
```

Replace the source vector construction in `discover_skill_entries`:

```rust
let sources = SkillSourceSet::from_options(options);
let mut by_name: HashMap<String, SkillEntry> = HashMap::new();
for source in sources.iter() {
    for entry in scan_source_dir(&source.root, source.scope, &source.label, &options.limits).await? {
        let should_insert = by_name
            .get(&entry.skill.name)
            .map(|existing| entry.skill.source_scope < existing.skill.source_scope)
            .unwrap_or(true);
        if should_insert {
            by_name.insert(entry.skill.name.clone(), entry);
        }
    }
}
```

- [ ] **Step 4: Export source module**

Modify `lib.rs`:

```rust
pub mod source;

pub use source::{SkillSource, SkillSourceSet};
```

- [ ] **Step 5: Run tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-skill runtime::tests::source_precedence_workspace_wins -- --nocapture
```

Expected:

```text
test result: ok.
```

## Task 4: Registry snapshot and reload

**Files:**
- Create: `macaca/crates/macaca-skill/src/snapshot.rs`
- Modify: `macaca/crates/macaca-skill/src/registry.rs`
- Modify: `macaca/crates/macaca-skill/src/lib.rs`

- [ ] **Step 1: Run GitNexus impact before editing `SkillRegistry`**

Use GitNexus:

```text
impact(target="SkillRegistry", direction="upstream", repo="agent")
```

Expected:

```text
Direct callers include macaca-web startup and integration tests.
```

- [ ] **Step 2: Add snapshot type**

Create `macaca/crates/macaca-skill/src/snapshot.rs`:

```rust
//! Skill registry snapshot primitives.

use serde::{Deserialize, Serialize};

use crate::definition::SkillDefinition;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillRegistrySnapshot {
    pub version: u64,
    pub skills: Vec<SkillDefinition>,
}

impl SkillRegistrySnapshot {
    pub fn new(mut skills: Vec<SkillDefinition>) -> Self {
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        Self { version: 1, skills }
    }

    pub fn len(&self) -> usize {
        self.skills.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}
```

- [ ] **Step 3: Add registry snapshot/reload API**

Modify `registry.rs`:

```rust
use crate::snapshot::SkillRegistrySnapshot;
```

Add methods:

```rust
    /// Capture executable skill definitions as a reloadable snapshot.
    pub fn snapshot(&self) -> SkillRegistrySnapshot {
        SkillRegistrySnapshot::new(self.skills.values().cloned().collect())
    }

    /// Replace current registry state from a snapshot.
    pub fn reload_from_snapshot(&mut self, snapshot: SkillRegistrySnapshot) {
        self.skills.clear();
        for skill in snapshot.skills {
            self.register(skill);
        }
    }
```

- [ ] **Step 4: Mark legacy direct loader APIs deprecated**

Annotate old direct APIs after snapshot/reload exists:

```rust
#[deprecated(note = "Use SkillRegistrySnapshot/reload primitives or a future SkillRegistryLoader facade for migration-safe loading.")]
pub async fn load_from_directory(&mut self, dir: impl AsRef<Path>) -> MacacaResult<usize> { ... }

#[deprecated(note = "Use SkillToolAdapter or collect through a tool exposure facade.")]
pub fn instantiate_tool(&self, name: &str) -> MacacaResult<SkillTool> { ... }

#[deprecated(note = "Use SkillToolAdapter or collect through a tool exposure facade.")]
pub fn instantiate_all_tools(&self) -> Vec<Box<dyn macaca_tools::Tool>> { ... }
```

- [ ] **Step 5: Add registry snapshot test**

Add to `registry.rs` tests:

```rust
#[test]
fn snapshot_and_reload_registry() {
    let mut reg = SkillRegistry::new();
    reg.register(make_skill("a"));
    reg.register(make_skill("b"));

    let snapshot = reg.snapshot();
    assert_eq!(snapshot.len(), 2);

    let mut restored = SkillRegistry::new();
    restored.reload_from_snapshot(snapshot);
    assert!(restored.get("a").is_some());
    assert!(restored.get("b").is_some());
}
```

- [ ] **Step 6: Export snapshot module**

Modify `lib.rs`:

```rust
pub mod snapshot;

pub use snapshot::SkillRegistrySnapshot;
```

- [ ] **Step 7: Run tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-skill registry::tests::snapshot_and_reload_registry -- --nocapture
```

Expected:

```text
test result: ok.
```

## Task 5: Tool adapter and runtime proxy

**Files:**
- Create: `macaca/crates/macaca-skill/src/adapter.rs`
- Modify: `macaca/crates/macaca-skill/src/tool.rs`
- Modify: `macaca/crates/macaca-skill/src/lib.rs`

- [ ] **Step 1: Run GitNexus impact before editing `SkillTool`**

Use GitNexus:

```text
impact(target="SkillTool", direction="upstream", repo="agent")
impact(target="execute_shell", direction="upstream", repo="agent")
```

Expected:

```text
Direct callers include SkillRegistry tool instantiation and macaca-skill tests.
```

- [ ] **Step 2: Add adapter/proxy primitives**

Create `macaca/crates/macaca-skill/src/adapter.rs`:

```rust
//! Executable skill tool adapter and runtime proxy.

use async_trait::async_trait;
use serde_json::Value;

use macaca_proto::{MacacaError, MacacaResult};

use crate::definition::{SkillDefinition, SkillEntryPoint};
use crate::tool::execute_shell_entry;

#[async_trait]
pub trait SkillRuntimeProxy: Send + Sync {
    async fn execute(&self, definition: &SkillDefinition, input: Value) -> MacacaResult<Value>;
}

#[derive(Debug, Clone, Default)]
pub struct LocalSkillRuntimeProxy;

#[async_trait]
impl SkillRuntimeProxy for LocalSkillRuntimeProxy {
    async fn execute(&self, definition: &SkillDefinition, input: Value) -> MacacaResult<Value> {
        match &definition.entry_point {
            SkillEntryPoint::ShellCommand { command, args } => {
                execute_shell_entry(command, args, &input).await
            }
            SkillEntryPoint::Script { path, interpreter } => {
                let cmd = interpreter.as_deref().unwrap_or("sh");
                execute_shell_entry(cmd, &[path.clone()], &input).await
            }
            SkillEntryPoint::McpServer { .. } => Err(MacacaError::Agent(
                "MCP skills should be loaded via Agent OS MCP runtime, not SkillTool".into(),
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkillToolAdapter {
    definition: SkillDefinition,
    runtime: LocalSkillRuntimeProxy,
}

impl SkillToolAdapter {
    pub fn local(definition: SkillDefinition) -> Self {
        Self {
            definition,
            runtime: LocalSkillRuntimeProxy,
        }
    }

    pub fn definition(&self) -> &SkillDefinition {
        &self.definition
    }

    pub async fn execute(&self, input: Value) -> MacacaResult<Value> {
        self.runtime.execute(&self.definition, input).await
    }
}
```

- [ ] **Step 3: Route `SkillTool` through adapter**

Modify `tool.rs`:

```rust
use crate::adapter::SkillToolAdapter;
```

Change `SkillTool`:

```rust
pub struct SkillTool {
    adapter: SkillToolAdapter,
}

impl SkillTool {
    #[deprecated(note = "Use SkillToolAdapter::local and tool exposure facades for new code.")]
    pub fn new(definition: SkillDefinition) -> Self {
        Self {
            adapter: SkillToolAdapter::local(definition),
        }
    }

    pub fn from_adapter(adapter: SkillToolAdapter) -> Self {
        Self { adapter }
    }
}
```

Update `Tool` impl:

```rust
fn name(&self) -> &str {
    &self.adapter.definition().name
}

fn description(&self) -> &str {
    &self.adapter.definition().description
}

fn parameters_schema(&self) -> Value {
    self.adapter.definition().parameters.clone()
}

async fn execute(&self, input: Value) -> MacacaResult<Value> {
    self.adapter.execute(input).await
}
```

Rename old private `execute_shell` to public crate-level helper:

```rust
pub(crate) async fn execute_shell_entry(
    command: &str,
    base_args: &[String],
    input: &Value,
) -> MacacaResult<Value> {
    ...
}
```

- [ ] **Step 4: Export adapter module**

Modify `lib.rs`:

```rust
pub mod adapter;

pub use adapter::{LocalSkillRuntimeProxy, SkillRuntimeProxy, SkillToolAdapter};
```

- [ ] **Step 5: Run tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-skill tool::tests -- --nocapture
```

Expected:

```text
test result: ok.
```

## Task 6: Runtime handle for provision lifecycle

**Files:**
- Create: `macaca/crates/macaca-skill/src/handle.rs`
- Modify: `macaca/crates/macaca-skill/src/provisioner.rs`
- Modify: `macaca/crates/macaca-skill/src/lib.rs`

- [ ] **Step 1: Run GitNexus impact before editing `SkillProvisioner`**

Use GitNexus:

```text
impact(target="SkillProvisioner", direction="upstream", repo="agent")
impact(target="provision_skill", direction="upstream", repo="agent")
```

Expected:

```text
Direct callers are macaca-skill tests and provisioner flows.
```

- [ ] **Step 2: Add handle/state primitives**

Create `macaca/crates/macaca-skill/src/handle.rs`:

```rust
//! Skill runtime lifecycle handle.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillRuntimeState {
    Installed,
    Provisioned,
    Active,
    Error(String),
    Released,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillRuntimeHandle {
    pub skill_id: String,
    pub client_id: String,
    pub target_dir: PathBuf,
    pub state: SkillRuntimeState,
}

impl SkillRuntimeHandle {
    pub fn provisioned(
        skill_id: impl Into<String>,
        client_id: impl Into<String>,
        target_dir: PathBuf,
    ) -> Self {
        Self {
            skill_id: skill_id.into(),
            client_id: client_id.into(),
            target_dir,
            state: SkillRuntimeState::Provisioned,
        }
    }

    pub fn released(mut self) -> Self {
        self.state = SkillRuntimeState::Released;
        self
    }
}
```

- [ ] **Step 3: Add additive provision handle API**

Modify `provisioner.rs`:

```rust
use crate::handle::SkillRuntimeHandle;
```

Add:

```rust
    /// Provision a specific skill and return a lifecycle handle.
    pub async fn provision_skill_with_handle(
        &self,
        skill_name: &str,
        client_name: &str,
    ) -> MacacaResult<SkillRuntimeHandle> {
        self.provision_skill(skill_name, client_name).await?;
        let client = self
            .clients
            .get(client_name)
            .ok_or_else(|| MacacaError::NotFound(format!("Unknown client: {client_name}")))?;
        Ok(SkillRuntimeHandle::provisioned(
            skill_name,
            client_name,
            client.skills_dir.join(skill_name),
        ))
    }
```

- [ ] **Step 4: Add provision handle test**

Add to `provisioner.rs` tests:

```rust
#[tokio::test]
async fn provision_skill_with_handle_returns_state() {
    let central = tempfile::tempdir().unwrap();
    let client = tempfile::tempdir().unwrap();
    let skill_dir = central.path().join("browser");
    tokio::fs::create_dir_all(&skill_dir).await.unwrap();
    tokio::fs::write(skill_dir.join("SKILL.md"), "---\nname: browser\ndescription: Browser\n---\nBody")
        .await
        .unwrap();

    let mut provisioner = SkillProvisioner::with_central_store(central.path().to_path_buf());
    provisioner.register_client(ClientConfig {
        name: "test-client".into(),
        skills_dir: client.path().to_path_buf(),
    });

    let handle = provisioner
        .provision_skill_with_handle("browser", "test-client")
        .await
        .unwrap();

    assert_eq!(handle.skill_id, "browser");
    assert_eq!(handle.client_id, "test-client");
    assert_eq!(handle.state, crate::handle::SkillRuntimeState::Provisioned);
    assert!(handle.target_dir.join("SKILL.md").exists());
}
```

- [ ] **Step 5: Export handle module**

Modify `lib.rs`:

```rust
pub mod handle;

pub use handle::{SkillRuntimeHandle, SkillRuntimeState};
```

- [ ] **Step 6: Run tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-skill provisioner::tests::provision_skill_with_handle_returns_state -- --nocapture
```

Expected:

```text
test result: ok.
```

## Task 7: Full verification and containment

**Files:**
- Modify: `macaca/openspec/changes/refactor-macaca-skill-patterns/tasks.md`

- [ ] **Step 1: Run OpenSpec validation**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
openspec validate refactor-macaca-skill-patterns --strict
```

Expected:

```text
Change 'refactor-macaca-skill-patterns' is valid
```

- [ ] **Step 2: Run crate tests**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-skill -- --nocapture
```

Expected:

```text
test result: ok.
```

- [ ] **Step 3: Run consumer compile check**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo check -p macaca-skill -p macaca-web -p macaca-runtime-host
```

Expected:

```text
Finished `dev` profile
```

- [ ] **Step 4: Run deprecated containment grep**

Run:

```bash
cd /Users/quantum/Code/dev/agent
rg -n "load_from_directory\\(|instantiate_all_tools\\(|SkillTool::new\\(" macaca/crates --glob '*.rs'
```

Expected:

```text
Only macaca-skill internals/tests and existing macaca-web startup compatibility path should remain.
```

- [ ] **Step 5: Run GitNexus change detection**

Use GitNexus:

```text
detect_changes(scope="all", repo="agent")
```

Expected:

```text
Affected symbols are limited to macaca-skill policy/source/snapshot/adapter/handle plus expected legacy wrapper methods.
```

- [ ] **Step 6: Update OpenSpec tasks**

After all checks pass, update `macaca/openspec/changes/refactor-macaca-skill-patterns/tasks.md` to mark all completed items as `- [x]`.

- [ ] **Step 7: Commit**

Run:

```bash
cd /Users/quantum/Code/dev/agent
git add docs/superpowers/plans/2026-05-02-refactor-macaca-skill.md \
  macaca/openspec/changes/refactor-macaca-skill-patterns \
  macaca/crates/macaca-skill
git commit -m "refactor: add skill runtime primitives"
gitnexus analyze
```

Expected:

```text
[branch <hash>] refactor: add skill runtime primitives
Repository indexed successfully
```

## Risk Notes

- `SkillRuntime::build_snapshot()` is called from real session construction and skill status APIs. Any behavior drift can break prompt skill catalog, snapshot reload, or skill-backed MCP discovery.
- Policy reason strings are user-visible through skill status APIs, so reason strings must remain stable.
- `SkillTool` executes local shell/script commands. Adapter extraction must not change timeout, stdout/stderr shape, argument handling, or working directory semantics.
- Provisioner must not take ownership of MCP/browser process lifecycle. Skill metadata can discover MCP definitions, but MCP runtime remains OS-level.
- Deprecated APIs must not be removed in this change because `macaca-web` still uses startup compatibility paths.

## Handoff

Plan complete. The next step is to create the OpenSpec change `refactor-macaca-skill-patterns`; implementation must wait for explicit approval after the proposal is written and reviewed.
