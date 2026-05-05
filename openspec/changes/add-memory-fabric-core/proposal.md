# Change: 增加 Macaca Memory Fabric 核心模型

## Why

Macaca 的记忆系统是 agent OS 的核心基础设施，需要同时支持每个 agent 的独家长期记忆和同一 session/project 下的共享项目记忆。当前 `macaca-memory` 已有基础 store、manager、isolated manager、facade、cache、snapshot、query strategy，但还缺少统一的强类型 scope、路由、provider/capability 边界和面向上层 application 的 MemoryFacade。

本变更建立 Memory Fabric 核心模型，使后续 provider、Milvus 向量拓扑、active recall、governance/knowledge layer 都能在同一抽象下自由装配。

## What Changes

- 在 `macaca-memory` 单 crate 内增加 Memory Fabric 核心模块，不新增额外 crate。
- 定义 `MemoryScope` 和 `MemoryVisibility`，显式表达 `AgentPrivate`、`SessionShared`、`ApplicationShared`、`UserScoped`、`GlobalSystem`。
- 定义 `MemoryFacade`、`MemoryRouter`、`MemoryProvider`、capability traits、lifecycle event DTO。
- 将现有 `IsolatedMemoryManager` 映射为 agent private memory 默认实现，将现有 session/file/vector manager 映射为 session shared/builtin provider 基础。
- 定义 agent private 与 session/project shared 的默认读写路由和隔离规则。
- 保留现有 public API；被新 facade 替代的旧入口后续只标记 deprecated，不删除。

## Impact

- Affected specs: `macaca-memory-fabric`
- Affected code:
  - `macaca/crates/macaca-memory/src/core/`
  - `macaca/crates/macaca-memory/src/lib.rs`
  - 现有 `facade.rs`、`manager.rs`、`isolated.rs`、`store.rs` 的 re-export 和兼容接入
- Compatibility:
  - 未显式配置时，现有基础记忆行为必须保持可用。
  - 不新建 `macaca-memory-core`、`macaca-memory-index` 等额外 crate。
- Follow-up changes:
  - `add-memory-vector-backend-topology`
  - `add-memory-provider-runtime`
  - `add-memory-active-recall-integration`
  - `add-memory-governance-knowledge-layer`
