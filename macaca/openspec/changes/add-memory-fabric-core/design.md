## Context

研究报告 `docs/memory-system-openclaw-hermes-research.md` 明确 Macaca 需要 Memory Fabric：上层 application 和 agent 不直接依赖某个具体记忆系统，而是通过统一 facade、scope、capability 和 runtime event 访问记忆。

Macaca 的核心需求包括：

- 每个 agent 拥有独立的 `AgentPrivate` 长期记忆。
- 同一 session/project 下多个 agent 拥有 `SessionShared` 项目共享记忆。
- 记忆系统必须可插拔、可组合、可替换。
- 记忆能力在 `macaca-memory` 单 crate 内通过目录模块组织，不新增多个 crate。

## Goals

- 建立 Memory Fabric 核心抽象。
- 强制所有新 memory 操作携带 `MemoryScope`。
- 明确 agent private 与 session/project shared 的隔离和路由。
- 保持现有 `macaca-memory` 行为兼容。
- 为 provider、vector backend、active recall、governance/knowledge layer 提供基础 contract。

## Non-Goals

- 不在本变更中实现远程 provider 协议。
- 不在本变更中实现完整 Milvus database/collection 管理。
- 不在本变更中实现 active recall pipeline。
- 不在本变更中实现 wiki/knowledge compiler。
- 不新增额外 crate。
- 不删除现有 manager/store/provider/vector trait。

## Decisions

### Decision 1: `macaca-memory` 保持唯一核心 crate

所有 Memory Fabric 核心能力放在 `macaca/crates/macaca-memory/src/core/` 下。

推荐结构：

```text
macaca-memory/src/core/
  mod.rs
  scope.rs
  facade.rs
  router.rs
  provider.rs
  capability.rs
  lifecycle.rs
  status.rs
```

理由：

- 避免 crate 过度拆分。
- 保持依赖方向简单。
- 文件超过 500 行时按职责拆子模块。

### Decision 2: `MemoryScope` 是所有新 memory 操作的主轴

`MemoryScope` 必须至少表达：

- `application_id`
- `agent_id` / `agent_name`
- `session_id` / `project_id`
- `tenant_id`
- `user_id`
- `namespace`
- `visibility`

`MemoryVisibility` 至少包含：

- `AgentPrivate`
- `SessionShared`
- `ApplicationShared`
- `UserScoped`
- `GlobalSystem`

规则：

- `AgentPrivate` 必须包含 application 与 agent 维度。
- `SessionShared` 必须包含 application 与 session/project 维度。
- provider 不得从全局状态推断 scope。

### Decision 3: 使用 Facade + Router + Strategy + Capability

设计模式：

- `MemoryFacade` 使用 Facade，作为上层唯一入口。
- `MemoryRouter` 使用 Strategy，根据 scope、visibility、policy 路由到 provider/capability。
- `MemoryProvider` 使用 Strategy/Factory 边界。
- capability traits 使用接口隔离原则，避免 provider 被迫实现所有能力。

核心能力：

- `MemoryStoreCapability`
- `MemorySearchCapability`
- `MemoryPromptCapability`
- `MemoryLifecycleCapability`
- `MemoryFlushCapability`
- `MemoryArtifactCapability`
- `MemoryGovernanceCapability`

### Decision 4: 现有 manager 通过 adapter 进入 Fabric

现有 `IsolatedMemoryManager` 是 `AgentPrivate` 的默认基础实现。

现有 `MemoryManager` 可作为 builtin/session shared 基础实现。

首轮不强行移动全部文件，只新增 core 模块和 adapter/re-export。后续可渐进迁移现有 `facade.rs`、`manager.rs`、`isolated.rs`。

## Risks / Trade-offs

- Risk: scope 设计过弱导致后续串记忆。
  - Mitigation: 在 spec 和测试中强制 `AgentPrivate` / `SessionShared` 必填维度。
- Risk: facade 过早替代所有旧 API 导致大范围迁移。
  - Mitigation: additive-first，旧 API 保留，后续迁移时标记 deprecated。
- Risk: capability 拆分过细造成首轮实现负担。
  - Mitigation: trait 可以有默认空实现或用 optional capability registry。

## Migration Plan

1. 新增 core DTO/trait/router/facade。
2. 包装现有 managers 为 builtin adapters。
3. 为 agent private 与 session shared 增加 contract tests。
4. 更新 `lib.rs` re-export。
5. 后续 change 接入 vector provider、remote provider、active recall、governance。
