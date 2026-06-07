# OpenClaw / Hermes 记忆系统研究报告

> 目标：研究 `/Users/quantum/code/dev/agent/openclaw` 与 `/Users/quantum/code/dev/agent/hermes-agent` 的记忆系统，为 Macaca 后续实现可插拔、自由装配、面向多 application / 多 agent 的记忆基础设施提供可操作参考。

## 1. 修订后的核心判断

Macaca 的记忆系统不是一个辅助功能，而是 agent OS 的核心基础设施。它必须服务于上层 application 中不同类型 agent 的长期自治、跨 session 工作延续、项目协作、上下文工程、可审计知识沉淀和第三方记忆系统接入。因此，设计时不应以“快速上线”或“避免拖慢开发速度”为主要取舍，而应以长期架构正确性、可插拔性、自由装配能力和可演进性为第一原则。

OpenClaw 的 `memory-core`、embedding provider、memory-wiki、active-memory、memory-lancedb 并不是 Macaca “暂时不用”的能力。它们代表的是完整记忆基础设施的多个必要维度：

- `memory-core`：默认本地记忆、文件可读性、索引可重建、基础检索工具。
- embedding provider：将记忆系统与向量化模型解耦，避免绑定单一厂商。
- memory-wiki：把原始记忆提升为结构化、可审计、带证据的知识层。
- active-memory：运行时主动召回，服务低延迟上下文注入。
- memory-lancedb：可替换向量库 / 长期语义记忆 backend 的参考实现。Macaca 当前长期记忆向量化默认采用 Milvus，因此应吸收其“可替换 backend”思想，而不是替换掉 Macaca 已有的 Milvus 拓扑。

Macaca 应把这些能力视为目标架构中的候选模块，而不是因为复杂就推迟思考。可以分阶段落地，但整体架构必须一开始允许这些模块自由组合。

## 2. Macaca 记忆系统的目标形态

Macaca 的记忆系统应定位为 **Memory Fabric**：一个可插拔、可组合、可替换的记忆总线。上层 application 和 agent 不直接依赖某个记忆实现，而是通过统一 facade、scope、capability 和 runtime event 与记忆系统交互。

### 2.1 必须支持的两类核心记忆

Macaca 上层 application 中会有多个 agent。每个 agent 都需要自己的独家记忆，同时一个 session / project 也需要共享记忆。

#### Agent Private Memory

每个 agent 拥有独立的长期记忆，默认不被其他 agent 读取。

用途：

- agent 自己的经验、偏好、失败案例、工具使用习惯。
- agent 对某类任务的策略沉淀。
- agent 的 persona、能力边界、长期目标和自我改进数据。
- application 中不同职责 agent 的专属知识，例如 planner、coder、reviewer、operator 各自的工作记忆。

隔离规则：

- 以 `application_id + agent_name/agent_id` 为最小隔离边界。
- 可选叠加 `tenant_id`、`user_id`、`namespace`。
- 默认情况下 agent A 不读取 agent B 的 private memory。
- 共享必须显式通过 project/session shared memory 或授权策略完成。

#### Session / Project Shared Memory

同一 session 或项目中的多个 agent 需要共享一部分项目级记忆。

用途：

- 当前项目目标、约束、决策、架构约定。
- 多 agent 协作状态。
- 已确认事实和不可重复踩坑事项。
- 用户在该 session / project 中明确要求所有 agent 记住的信息。
- 跨 agent handoff 的稳定上下文。

隔离规则：

- 以 `application_id + session_id` 或 `application_id + project_id` 为共享边界。
- 同一个 session / project 中的授权 agent 可读写。
- 共享记忆必须带 provenance，记录由哪个 agent、哪个 turn、哪个 tool 或哪个用户输入产生。
- 私有记忆晋升到共享记忆必须通过明确策略，不允许默认自动泄露。

### 2.2 记忆层级

建议 Macaca 记忆系统至少建模以下层级：

| 层级 | Scope | 主要用途 | 默认可见性 |
| --- | --- | --- | --- |
| Working Memory | session + agent | 当前 turn / 当前窗口内短期上下文 | 当前 agent |
| Agent Private Memory | application + agent | agent 独有长期经验与偏好 | 当前 agent |
| Session Shared Memory | application + session/project | 多 agent 项目共享事实、决策、约束 | session 内授权 agent |
| Application Memory | application | application 长期通用知识 | application 内授权 agent |
| User Memory | tenant/user | 用户偏好、长期关系、身份上下文 | 受权限控制 |
| Knowledge / Wiki Memory | application/project/user | 结构化知识、claim、evidence、矛盾检测 | 按 namespace 授权 |
| External Memory | provider-defined | Mem0、Honcho、企业 RAG、向量库等 | 由 adapter 映射到 Macaca scope |

这比简单的 `session_id` 主键更合适。`session_id` 是重要维度，但不能成为唯一记忆边界。Macaca 需要的是多维 scope。

## 3. 参考项目范围

### OpenClaw 关键路径

- `extensions/memory-core/index.ts`
- `extensions/memory-core/openclaw.plugin.json`
- `extensions/memory-core/src/memory/manager.ts`
- `extensions/memory-core/src/memory/*`
- `packages/memory-host-sdk/src/host/openclaw-runtime-memory.ts`
- `packages/memory-host-sdk/src/host/openclaw-runtime.ts`
- `src/plugins/memory-state.ts`
- `src/plugins/registry.ts`
- `src/plugins/config-activation-shared.ts`
- `src/plugins/memory-embedding-providers.ts`
- `extensions/memory-lancedb/index.ts`
- `extensions/active-memory/index.ts`
- `extensions/memory-wiki/src/*`
- `docs/concepts/memory.md`

### Hermes 关键路径

- `agent/memory_provider.py`
- `agent/memory_manager.py`
- `plugins/memory/__init__.py`
- `plugins/memory/honcho/__init__.py`
- `plugins/memory/mem0/__init__.py`
- `plugins/memory/supermemory/__init__.py`
- `plugins/memory/holographic/*`
- `plugins/memory/hindsight/__init__.py`
- `plugins/memory/openviking/__init__.py`
- `plugins/memory/retaindb/__init__.py`
- `plugins/memory/byterover/__init__.py`
- `run_agent.py`
- `website/docs/developer-guide/memory-provider-plugin.md`
- `website/docs/user-guide/features/memory-providers.md`

### Macaca 当前相关基础

- `macaca/crates/macaca-memory/src/store.rs`
- `macaca/crates/macaca-memory/src/manager.rs`
- `macaca/crates/macaca-memory/src/isolated.rs`
- `macaca/crates/macaca-memory/src/backend.rs`
- `macaca/crates/macaca-memory/src/cache.rs`
- `macaca/crates/macaca-memory/src/snapshot.rs`
- `macaca/crates/macaca-memory/src/query.rs`
- `macaca/crates/macaca-context/src/*`
- `macaca/crates/macaca-framework/src/memory.rs`
- `macaca/docs/design-pattern-refactor-plans/macaca-memory.md`

Macaca 当前已经有 `MemoryStore`、`MemoryRetriever`、`EmbeddingProvider`、`VectorStore`、`MemoryManager`、`IsolatedMemoryManager`、facade、cache、backend、snapshot、query strategy 等基础。后续应在这些基础上建立更强的 provider/runtime/scope/capability 架构。

当前 Macaca 长期记忆向量化已经采用 Milvus，并且有一个重要拓扑约定：

- 一个 `application` 对应一个向量数据库 database。
- 该 database 下每个 `agent` 对应一个 collection。
- collection 相当于关系型数据库中的表，是 agent 私有长期向量记忆的默认隔离单元。

这个拓扑不应被视为 Milvus 专属细节，而应上升为 Macaca 的 `VectorMemoryBackend` 架构 contract。Milvus 是默认实现，用户可以替换为其他支持同等拓扑语义的向量数据库或远程向量服务。

## 4. OpenClaw 记忆系统分析

### 4.1 Slot 与 Capability

OpenClaw 的记忆系统是 slot 驱动的。

关键点：

- `plugins.slots.memory` 决定哪个插件拥有主记忆槽位。
- 默认槽位是 `memory-core`。
- 插件 manifest 使用 `kind: "memory"` 表明它可以成为 memory 插件。
- 被选中的 memory 插件通过 `api.registerMemoryCapability(...)` 注册能力。
- 非 memory 插件可以通过 contract 注册 embedding provider。
- 多 kind 插件只有在被选中为 memory slot 时才能注册主 memory runtime。

这套机制对 Macaca 很有价值，因为它解决了“默认系统”和“可替换系统”之间的边界问题。Macaca 应借鉴 slot，但不能只支持一个粗粒度 slot。更适合的模型是：

- 主记忆 runtime slot：决定默认读写路径。
- agent private memory slot：每个 agent 可覆盖自己的私有记忆 provider。
- session shared memory slot：项目共享记忆可单独选择 provider。
- embedding provider slot：向量化能力独立选择。
- vector memory backend slot：长期向量记忆 backend 独立选择，默认 Milvus，必须支持 application database + agent collection 的拓扑语义。
- knowledge compiler slot：wiki / knowledge graph / claim store 可独立选择。
- active recall slot：主动召回策略可独立选择。

这样可以形成自由装配，而不是“一个 provider 包办所有能力”。

### 4.2 `MemoryPluginCapability`

OpenClaw 的 capability 包含：

- `promptBuilder`
- `flushPlanResolver`
- `runtime`
- `publicArtifacts`

这个设计说明 memory provider 不只是 store，还要参与上下文生成、压缩前 flush、运行时检索、公共 artifact 管理。Macaca 应进一步扩展为多个可组合 capability：

```rust
pub struct MemoryCapabilities {
    pub store: Option<Arc<dyn MemoryStoreCapability>>,
    pub search: Option<Arc<dyn MemorySearchCapability>>,
    pub prompt: Option<Arc<dyn MemoryPromptCapability>>,
    pub lifecycle: Option<Arc<dyn MemoryLifecycleCapability>>,
    pub flush: Option<Arc<dyn MemoryFlushCapability>>,
    pub artifacts: Option<Arc<dyn MemoryArtifactCapability>>,
    pub governance: Option<Arc<dyn MemoryGovernanceCapability>>,
}
```

好处：

- 一个 provider 可以只提供 store。
- 一个 provider 可以只提供 active recall。
- 一个 provider 可以只提供 wiki supplement。
- 一个 provider 可以组合多个能力。
- 用户可以自由装配，不被迫使用一体化系统。

### 4.3 `memory-core`

OpenClaw `memory-core` 包含：

- Markdown 长期记忆。
- daily notes。
- SQLite / FTS / vector index。
- `memory_search` / `memory_get` 工具。
- flush plan。
- dreaming。
- public artifacts。

Macaca 不应把 `memory-core` 理解成“太复杂暂时不用”。相反，它体现了默认记忆系统应该具备的完整能力：

- 人类可读的 source of truth。
- 可重建索引。
- 关键词检索与向量检索并存。
- 工具化访问。
- 压缩前记忆 flush。
- 后台整理与长期记忆治理。
- 可展示和可审计的 artifact。

Macaca 可以不用照搬 OpenClaw 的 TypeScript loader，但应该吸收这些能力并在 `macaca-memory` 单 crate 内模块化。这里不建议额外新开多个 crate；先通过 `macaca/crates/macaca-memory/src/...` 下的文件结构保持边界清晰：

- `src/core/`：默认本地记忆 runtime、provider/capability/facade/router 核心抽象。
- `src/index/`：FTS / vector / hybrid index、query strategy、rerank、metadata filter。
- `src/tools/`：标准记忆工具定义与适配，例如 `memory_search`、`memory_get`、`memory_store`、`memory_delete`。
- `src/governance/`：候选、晋升、审计、删除、tombstone、PII propagation、promotion policy。
- `src/artifacts/`：可读文件、报告、wiki 输出、public artifacts。
- `src/providers/`：builtin、remote、mcp、milvus、lancedb、qdrant 等 provider/adapter 实现。
- `src/embedding/` 或保留现有 `embedding.rs` 后续目录化：embedding provider registry、cache、retry、batch、timeout。
- `src/vector/` 或保留现有 `vector.rs` 后续目录化：`VectorMemoryBackend` contract 与 Milvus 默认实现。

文件是否立即目录化可以分阶段执行，但架构文档和后续 OpenSpec 不应再引导拆成多个独立 crate。

### 4.4 Embedding Provider

OpenClaw 将 embedding provider 从 memory backend 中解耦，这一点 Macaca 必须借鉴。

原因：

- 用户可能使用 DashScope、OpenAI-compatible、Ollama、Voyage、Mistral、自研 embedding。
- 记忆 store 不应该知道 embedding 厂商细节。
- 相同 embedding provider 可服务多个 memory backend。
- embedding 缓存、限流、批处理、fallback 是横切能力。

Macaca 建议：

```rust
#[async_trait::async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn id(&self) -> &str;
    fn dimensions(&self) -> usize;
    async fn embed(&self, input: EmbeddingRequest) -> MacacaResult<EmbeddingResponse>;
}

pub trait EmbeddingProviderFactory: Send + Sync {
    fn provider_id(&self) -> &str;
    fn create(&self, config: &EmbeddingProviderConfig) -> MacacaResult<Arc<dyn EmbeddingProvider>>;
}
```

再通过 decorator 叠加：

- cache。
- timeout。
- retry。
- circuit breaker。
- metrics。
- batch。

### 4.5 Memory Wiki

OpenClaw `memory-wiki` 的核心价值不是“另一个检索工具”，而是把原始记忆提升为结构化知识层：

- claim。
- evidence。
- contradiction。
- freshness。
- dashboards。
- compiled digests。
- wiki-native tools。

Macaca 需要类似能力。原因是 agent OS 长期运行后，原始 conversation memory 会越来越乱，仅靠向量检索无法保证知识质量。必须有一个结构化记忆治理层。

Macaca 可设计为：

- 原始记忆层：存事实、turn、事件、摘要、来源。
- 编译知识层：把原始记忆编译为 claim / decision / preference / constraint。
- 审计层：记录证据、冲突、新旧版本。
- 消费层：context engine 和 agent tools 只读取高质量编译结果，必要时回溯原始证据。

### 4.6 Active Memory

OpenClaw `active-memory` 对 Macaca 很重要，因为它对应“运行时主动召回”。不是所有记忆都应通过 agent 显式调用 `memory_search` 才出现。上下文工程需要在模型调用前主动检索有价值的记忆。

Macaca 应把 active memory 作为独立策略：

- 输入：当前 user message、recent turns、agent role、session goal、application context。
- 输出：短小、可解释、有来源的 memory prefetch。
- 约束：token budget、latency budget、privacy policy、scope policy。
- 可替换：规则检索、向量检索、LLM subagent、QMD、远程系统。

### 4.7 Memory LanceDB

OpenClaw `memory-lancedb` 的价值是展示“可替换长期语义记忆 backend”：

- storage backend 可替换。
- embedding provider 可配置。
- auto-capture / auto-recall 可配置。
- 工具可标准化。

Macaca 不应锁定 LanceDB，但应提供同类插拔点：

- builtin sqlite/vector。
- milvus adapter，默认实现。
- lancedb adapter。
- qdrant adapter。
- remote vector adapter。
- enterprise RAG adapter。

需要强调的是：Macaca 不绑定 Milvus 这个供应商，但需要保持当前 Milvus 拓扑表达出来的架构概念。默认长期向量记忆是：

```text
Application
  └── Vector Database
        ├── Agent A Collection
        ├── Agent B Collection
        └── Agent C Collection
```

这对应 agent private memory 的默认物理隔离。其他向量数据库接入时不必使用完全相同的 API 名称，但必须能映射出同等语义：

- `application_id` 能映射到 database、namespace、tenant、project、index prefix 或等价隔离域。
- `agent_id` / `agent_name` 能映射到 collection、table、class、partition、namespace 或等价隔离单元。
- 单 agent collection 的 schema 必须支持 memory id、scope、content、vector、metadata、created_at、updated_at、source/provenance。
- 查询必须默认限制在当前 agent collection，除非显式走 session shared / application shared 路由。
- 删除、重建索引、迁移和备份必须以 application database 和 agent collection 为基本操作单位。

因此，Macaca 需要的是 `VectorMemoryBackend` contract，而不是“Milvus-only”实现。

## 5. Hermes 记忆系统分析

### 5.1 `MemoryProvider` 生命周期

Hermes 的 `MemoryProvider` 是最适合学习的“小白可实现 provider contract”。

它定义：

- `is_available()`
- `initialize(session_id, **kwargs)`
- `system_prompt_block()`
- `prefetch(query, session_id=...)`
- `queue_prefetch(query, session_id=...)`
- `sync_turn(user, assistant, session_id=...)`
- `get_tool_schemas()`
- `handle_tool_call(...)`
- `on_turn_start(...)`
- `on_session_end(messages)`
- `on_session_switch(...)`
- `on_pre_compress(messages)`
- `on_memory_write(...)`
- `on_delegation(...)`
- `shutdown()`
- `get_config_schema()`
- `save_config(...)`

Macaca 应借鉴生命周期完整性，但 Rust 实现应拆分为多个 trait，避免单个 provider 必须实现所有方法。

### 5.2 Provider Discovery 与配置

Hermes 扫描：

- 内置 `plugins/memory/<name>/`
- 用户 `$HERMES_HOME/plugins/<name>/`

配置：

```yaml
memory:
  provider: mem0
```

小白用户可以通过 setup wizard 选择 provider、填必要字段。这个体验 Macaca 必须保留：高级架构可以复杂，但用户替换默认记忆系统的体验必须简单。

Macaca 推荐三种接入方式：

- 内置 provider：`macaca-memory` crate 内置模块。
- 远程 provider：HTTP/gRPC/MCP 协议，无需写 Rust。
- 高级本地插件：后续可考虑 WASM 或受控动态插件。

### 5.3 MemoryManager

Hermes `MemoryManager` 的价值：

- provider 失败不阻断主流程。
- tool schema 集中收集和路由。
- lifecycle hook 统一分发。
- external provider 最多一个，避免冲突。

Macaca 应保留“失败隔离”和“工具路由”，但需要比 Hermes 更强：

- 支持 agent private + session shared 双写双读。
- 支持多个 supplement。
- 支持 provider 组合。
- 支持 scope policy。
- 支持异步事件总线。
- 支持 trace / metrics / diagnostics。

### 5.4 Honcho 的启发

Honcho 的重点不是 SDK，而是 identity / peer / session strategy：

- user peer。
- AI peer。
- workspace。
- profile isolation。
- per-directory / per-repo / per-session / global session strategy。
- base context 与 dialectic reasoning 分层。
- cadence 控制成本。

Macaca 应抽象出通用 identity model：

```rust
pub struct MemoryIdentity {
    pub tenant_id: Option<String>,
    pub user_id: Option<String>,
    pub application_id: String,
    pub agent_id: Option<String>,
    pub agent_name: Option<String>,
    pub session_id: Option<String>,
    pub project_id: Option<String>,
}
```

agent private memory 和 session shared memory 都基于这个 identity/scope 派生，不绑定 Honcho peer 概念。

### 5.5 Mem0 的启发

Mem0 provider 值得借鉴：

- `is_available()` 只检查配置，不做网络调用。
- env + profile config 合并。
- circuit breaker。
- user scope 和 agent scope 分开。
- server-side extraction 与 semantic search。

Macaca 必须在所有外部 provider adapter 上标配：

- timeout。
- circuit breaker。
- retry policy。
- failure diagnostics。
- secret redaction。
- scope mapping。

### 5.6 Supermemory 的启发

Supermemory 值得借鉴：

- setup 极简，高级配置放文件。
- auto recall / auto capture 独立开关。
- multi-container。
- trivial message 过滤。
- entity context 限长。
- capture mode。

Macaca 的自动记忆应默认严谨，但不是默认弱。推荐：

- 自动捕获写入候选层，而不是直接写长期层。
- 明确用户“记住”可直接写入高置信层。
- agent 自我总结可写 agent private candidate。
- session 项目决策可写 shared candidate。
- promotion 策略可替换。

## 6. Macaca 推荐架构

### 6.0 代码组织原则

记忆系统后续能力会很多，但不应因此在当前阶段拆出一堆新 crate。推荐原则：

- 保持 `macaca-memory` 作为记忆系统唯一核心 crate。
- 在 `macaca-memory/src/` 下通过目录和模块边界组织复杂能力。
- 上层 crate 只依赖 `macaca-memory` 暴露的 facade / trait / DTO，不依赖内部目录实现。
- 单文件超过项目约定的 500 行时，优先按职责拆到同目录子模块，而不是新建 crate。
- 只有当某个模块拥有明确独立发布、独立版本、独立依赖生命周期时，才考虑未来拆 crate；当前记忆系统设计阶段不做这个拆分。

建议目标结构：

```text
macaca/crates/macaca-memory/src/
  lib.rs
  core/
    facade.rs
    provider.rs
    capability.rs
    router.rs
    scope.rs
    lifecycle.rs
  index/
    mod.rs
    fts.rs
    hybrid.rs
    query.rs
    rerank.rs
  vector/
    mod.rs
    backend.rs
    milvus.rs
    memory_topology.rs
    in_memory.rs
  embedding/
    mod.rs
    registry.rs
    cache.rs
    dashscope.rs
    mock.rs
  tools/
    mod.rs
    search.rs
    get.rs
    store.rs
    delete.rs
  governance/
    mod.rs
    candidate.rs
    promotion.rs
    audit.rs
    tombstone.rs
  artifacts/
    mod.rs
    markdown.rs
    report.rs
    wiki.rs
  providers/
    mod.rs
    builtin.rs
    remote.rs
    mcp.rs
```

当前已有的 `backend.rs`、`cache.rs`、`embedding.rs`、`facade.rs`、`file.rs`、`isolated.rs`、`manager.rs`、`query.rs`、`session.rs`、`snapshot.rs`、`store.rs`、`vector.rs` 可以渐进迁移到这些目录中。迁移时保持 public API 兼容，先加新模块 re-export，再逐步移动实现，避免一次性大搬迁。

### 6.1 总体分层

```text
Application / Agent
        |
MemoryFacade
        |
MemoryRouter
        |
+----------------------+-----------------------+
| Agent Private Memory | Session Shared Memory |
+----------------------+-----------------------+
        |
Memory Runtime Slots / Capability Registry
        |
+---------+---------+----------+-----------+
| Store   | Search  | Active   | Knowledge |
| Backend | Index   | Recall   | Compiler  |
+---------+---------+----------+-----------+
        |
Builtin / Milvus / Remote / MCP / Mem0 / Honcho / LanceDB / Enterprise RAG
```

### 6.2 Scope 模型

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MemoryScope {
    pub tenant_id: Option<String>,
    pub application_id: String,
    pub agent_id: Option<String>,
    pub agent_name: Option<String>,
    pub session_id: Option<String>,
    pub project_id: Option<String>,
    pub user_id: Option<String>,
    pub namespace: Option<String>,
    pub visibility: MemoryVisibility,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryVisibility {
    AgentPrivate,
    SessionShared,
    ApplicationShared,
    UserScoped,
    GlobalSystem,
}
```

关键规则：

- 所有 write/search/get/delete 请求必须携带 `MemoryScope`。
- `AgentPrivate` 必须包含 agent 维度。
- `SessionShared` 必须包含 session 或 project 维度。
- provider 不得从全局变量猜 scope。
- scope mapping 是 adapter 的显式职责。

### 6.3 MemoryFacade

上层 runtime、agent、framework、context engine 只依赖 facade。

```rust
#[async_trait::async_trait]
pub trait MemoryFacade: Send + Sync {
    async fn remember(&self, req: MemoryWriteRequest) -> MacacaResult<MemoryWriteResult>;
    async fn search(&self, req: MemorySearchRequest) -> MacacaResult<Vec<MemoryHit>>;
    async fn get(&self, req: MemoryGetRequest) -> MacacaResult<Option<MemoryDocument>>;
    async fn delete(&self, req: MemoryDeleteRequest) -> MacacaResult<()>;
    async fn prefetch(&self, req: MemoryPrefetchRequest) -> MacacaResult<MemoryPrefetchResult>;
    async fn status(&self, scope: MemoryScope) -> MacacaResult<MemoryStatusReport>;
}
```

### 6.4 MemoryRouter

MemoryRouter 根据 scope 和策略决定访问哪些 provider。

示例：

- agent private write → agent private provider。
- session shared write → session shared provider。
- recall for agent → agent private + session shared + optional application memory。
- context preflight → active recall strategy 读取多个 scope，按 budget 合并。
- explicit memory_search tool → 允许用户指定 corpus / visibility。

```rust
pub enum MemoryRoute {
    AgentPrivate,
    SessionShared,
    ApplicationShared,
    UserScoped,
    Supplements(Vec<String>),
    Composite(Vec<MemoryRoute>),
}
```

### 6.5 Provider 与 Capability

```rust
#[async_trait::async_trait]
pub trait MemoryProvider: Send + Sync {
    fn id(&self) -> &str;
    fn metadata(&self) -> MemoryProviderMetadata;
    fn capabilities(&self) -> MemoryProviderCapabilities;

    async fn initialize(&self, ctx: MemoryInitContext) -> MacacaResult<()>;
    async fn status(&self, ctx: MemoryStatusContext) -> MacacaResult<MemoryProviderStatus>;
    async fn shutdown(&self) -> MacacaResult<()>;
}
```

能力拆分：

```rust
#[async_trait::async_trait]
pub trait MemoryStoreCapability: Send + Sync {
    async fn write(&self, req: MemoryWriteRequest) -> MacacaResult<MemoryWriteResult>;
    async fn get(&self, req: MemoryGetRequest) -> MacacaResult<Option<MemoryDocument>>;
    async fn delete(&self, req: MemoryDeleteRequest) -> MacacaResult<()>;
}

#[async_trait::async_trait]
pub trait MemorySearchCapability: Send + Sync {
    async fn search(&self, req: MemorySearchRequest) -> MacacaResult<Vec<MemoryHit>>;
}

#[async_trait::async_trait]
pub trait ActiveRecallCapability: Send + Sync {
    async fn prefetch(&self, req: MemoryPrefetchRequest) -> MacacaResult<MemoryPrefetchResult>;
}

#[async_trait::async_trait]
pub trait KnowledgeCompileCapability: Send + Sync {
    async fn compile(&self, req: MemoryCompileRequest) -> MacacaResult<MemoryCompileReport>;
}
```

### 6.6 组件自由装配

配置不应只能选择一个 `provider` 包办所有能力。推荐支持 profile：

```toml
[memory]
profile = "default"

[memory.profiles.default]
agent_private_provider = "builtin"
session_shared_provider = "builtin"
active_recall = "hybrid"
knowledge_compiler = "wiki"
embedding_provider = "dashscope"
vector_backend = "milvus"

[memory.providers.builtin]
store = "sqlite"
source = "markdown"
index = "hybrid"

[memory.vector_backends.milvus]
kind = "milvus"
endpoint = "http://localhost:19530"
application_database_template = "{application_id}"
agent_collection_template = "{agent_name}"
dimension = 1536

[memory.providers.remote-company-rag]
kind = "remote"
endpoint = "https://memory.internal.example.com"
auth_env = "COMPANY_MEMORY_TOKEN"

[memory.agents.coder]
private_provider = "lancedb"

[memory.sessions.default]
shared_provider = "remote-company-rag"
```

这样可以做到：

- 某个 agent 使用 LanceDB 私有记忆。
- session 共享记忆使用企业 RAG。
- embedding 使用 DashScope。
- 长期向量记忆默认使用 Milvus，并保持 application database + agent collection 拓扑。
- active recall 使用本地 hybrid。
- knowledge compiler 使用 wiki。

这才符合“自由装配”的基础设施目标。

### 6.7 Vector Memory Backend Contract

Macaca 当前 Milvus 实现中的 `application → database`、`agent → collection` 应成为长期向量记忆的标准拓扑。

建议抽象为：

```rust
#[derive(Debug, Clone)]
pub struct VectorMemoryTopology {
    pub application_database: String,
    pub agent_collection: String,
}

#[async_trait::async_trait]
pub trait VectorMemoryBackend: Send + Sync {
    fn id(&self) -> &str;

    async fn ensure_application_database(
        &self,
        application_id: &str,
    ) -> MacacaResult<VectorDatabaseHandle>;

    async fn ensure_agent_collection(
        &self,
        database: &VectorDatabaseHandle,
        agent_id: &str,
        schema: VectorCollectionSchema,
    ) -> MacacaResult<VectorCollectionHandle>;

    async fn upsert_memory_vector(
        &self,
        collection: &VectorCollectionHandle,
        record: VectorMemoryRecord,
    ) -> MacacaResult<()>;

    async fn search_agent_collection(
        &self,
        collection: &VectorCollectionHandle,
        query: VectorSearchQuery,
    ) -> MacacaResult<Vec<VectorMemoryHit>>;

    async fn delete_memory_vector(
        &self,
        collection: &VectorCollectionHandle,
        memory_id: &MemoryId,
    ) -> MacacaResult<()>;
}
```

对 Milvus 的默认映射：

- `ensure_application_database(application_id)` → 创建或选择 Milvus database。
- `ensure_agent_collection(database, agent_id, schema)` → 在该 database 下创建或选择 agent collection。
- `upsert_memory_vector` → 向 agent collection 写入向量与 payload。
- `search_agent_collection` → 只在当前 agent collection 内检索。
- session shared memory 若需要向量化，可以使用独立 collection，例如 `session_{session_id}` 或 `project_{project_id}`，但必须通过 `MemoryVisibility::SessionShared` 显式路由，不能混入 agent private collection。

替代 backend 的合规要求：

- Qdrant 可以将 application 映射为 collection prefix / shard / tenant，将 agent 映射为 payload partition 或独立 collection，但必须通过 contract 保证隔离。
- LanceDB 可以将 application 映射为 database path，将 agent 映射为 table。
- 远程 backend 可以内部自由实现，但协议层必须暴露 topology metadata，证明其支持 application 与 agent 两级隔离。
- 如果某 backend 只能提供单 namespace 扁平存储，不能直接作为默认长期向量记忆 backend，只能作为 supplement 或 remote RAG adapter 使用。

## 7. 记忆写入与读取策略

### 7.1 写入路径

建议拆分为：

- explicit write：用户或 agent 明确要求记住。
- implicit candidate：自动捕获候选。
- system event：任务、工具、决策、错误、handoff 事件。
- promotion：候选晋升为长期记忆。
- mirror：内置记忆写入同步到外部 provider。

写入目标：

- agent 自我经验 → AgentPrivate。
- 项目事实 / 决策 / 约束 → SessionShared。
- 用户偏好 → UserScoped。
- application 通用知识 → ApplicationShared。
- 可证明知识 → Knowledge/Wiki。

### 7.2 读取路径

一个 agent 构建上下文时，建议按顺序读取：

1. Working memory。
2. AgentPrivate high-confidence recall。
3. SessionShared project recall。
4. ApplicationShared constraints。
5. UserScoped preference recall。
6. Knowledge/Wiki compiled facts。
7. External supplements。

MemoryRouter 应按 token budget、latency budget 和 relevance 合并。

### 7.3 冲突处理

记忆会冲突。Macaca 应提供：

- freshness。
- confidence。
- source provenance。
- visibility。
- conflict group。
- supersedes。
- revoked / deleted tombstone。

结构示例：

```rust
pub struct MemoryRecord {
    pub id: MemoryId,
    pub scope: MemoryScope,
    pub content: String,
    pub kind: MemoryKind,
    pub confidence: f32,
    pub source: MemorySource,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub supersedes: Vec<MemoryId>,
    pub conflict_group: Option<String>,
}
```

## 8. 小白用户替换默认记忆系统

架构可以强大，但替换路径必须简单。

### 8.1 远程 Provider

```toml
[memory]
profile = "remote-simple"

[memory.profiles.remote-simple]
agent_private_provider = "remote"
session_shared_provider = "remote"

[memory.providers.remote]
endpoint = "http://localhost:8787"
auth_env = "MACACA_MEMORY_TOKEN"
protocol = "macaca-memory-v1"
```

最小协议：

- `GET /memory/v1/status`
- `POST /memory/v1/search`
- `POST /memory/v1/get`
- `POST /memory/v1/write`
- `POST /memory/v1/delete`
- `POST /memory/v1/events`

所有请求必须携带 `scope`，这样远程系统也能理解 agent private 与 session shared 的区别。

### 8.2 MCP Provider

适合已有 MCP memory server：

```toml
[memory.providers.mcp]
server = "my-memory"
search_tool = "memory_search"
get_tool = "memory_get"
write_tool = "memory_store"
```

### 8.3 内置行业 Provider

```toml
[memory.providers.mem0]
api_key_env = "MEM0_API_KEY"
scope_mapping = "macaca-default"

[memory.providers.honcho]
api_key_env = "HONCHO_API_KEY"
workspace_template = "{application_id}"
agent_peer_template = "{agent_name}"
```

## 9. 应避免的问题

### 9.1 避免把复杂度当成理由削弱记忆系统

记忆系统是核心系统，值得投入。复杂能力可以模块化、分阶段实现，但不能在架构上假设“以后再想”。如果第一版 scope、capability、provider 边界设计不对，后续会被迫大改。

### 9.2 避免单 provider 包办所有能力

OpenClaw `memory-core` 能力完整，但 Macaca 更应该做成可组合模块，而不是一个巨型实现。

### 9.3 避免 session id 作为唯一主键

session 是共享记忆的重要维度，但 agent private memory 需要 application + agent 维度，用户记忆需要 user 维度，知识层需要 namespace / project 维度。所有记忆必须使用强类型 scope。

### 9.4 避免私有记忆泄漏到共享记忆

agent private memory 默认只能被该 agent 使用。晋升到 session shared 必须由 policy、用户指令或明确事件触发，并记录 provenance。

### 9.5 避免所有记忆直接塞 prompt

记忆系统应该支持：

- 显式工具检索。
- preflight active recall。
- context report。
- wiki digest。
- scoped exact get。

不是把所有命中都放进 system prompt。

## 10. 推荐实施路线

### Phase 1：Memory Fabric 基础模型

目标：建立不会返工的核心抽象。

任务：

- 定义 `MemoryScope`，显式支持 `AgentPrivate` 与 `SessionShared`。
- 定义 `MemoryFacade`。
- 定义 `MemoryRouter`。
- 定义 provider / capability trait。
- 定义 agent private 与 session shared 的路由规则。
- 将现有 `IsolatedMemoryManager` 映射到 AgentPrivate。
- 将现有 session/file/vector manager 映射到 SessionShared / builtin provider。

验收：

- 任意 memory 请求必须携带 scope。
- agent private 与 session shared 测试隔离通过。
- 未配置时默认 builtin 行为可用。

### Phase 2：可插拔 Provider 与自由装配

目标：支持不同能力使用不同 provider。

任务：

- 实现 `MemoryProviderRegistry`。
- 实现 profile 配置。
- 支持 agent private provider 与 session shared provider 分别配置。
- 实现 remote provider adapter。
- 实现 provider status / diagnostics。

验收：

- coder agent 可以使用一个 private provider。
- session shared 可以使用另一个 provider。
- provider 失败不阻断主任务，trace 可见。

### Phase 3：Embedding / Index / Backend 解耦

目标：吸收 OpenClaw embedding provider 与 memory-lancedb 的优点。

任务：

- `EmbeddingProviderRegistry`。
- embedding decorator：cache、timeout、retry、metrics。
- builtin keyword / FTS / vector / hybrid search。
- `VectorMemoryBackend` contract：默认 Milvus，强制表达 application database + agent collection 拓扑。
- 可插拔 vector backend：Milvus、builtin、LanceDB、Qdrant、remote。
- query strategy：keyword、vector、hybrid、filtered、rerank。

验收：

- 不同 agent 可使用不同 embedding/backend。
- 默认 Milvus backend 中，一个 application 创建或映射一个 database，每个 agent 创建或映射一个 collection。
- 替代 backend 必须通过 contract 测试证明能等价表达 application 隔离域和 agent collection 隔离单元。
- 无 embedding 时仍可检索。
- embedding 失败不影响写入。

### Phase 4：Active Memory

目标：实现运行时主动召回。

任务：

- `ActiveRecallCapability`。
- preflight recall pipeline。
- token budget 与 latency budget。
- agent private + session shared 合并策略。
- context report 记录每条召回来源。

验收：

- 创建 LLM prompt 前能自动召回 relevant memory。
- UI / trace 能看到 recall 来源、scope、耗时。

### Phase 5：Knowledge / Wiki Layer

目标：引入结构化知识治理。

任务：

- `KnowledgeCompileCapability`。
- claim / evidence / contradiction / freshness。
- project decision log。
- wiki digest。
- memory public artifacts。
- exact citation 回溯。

验收：

- 原始记忆可编译为结构化知识。
- 冲突知识可检测。
- context engine 优先消费高质量 digest。

### Phase 6：Governance 与长期自治

目标：支撑 7x24 agent OS。

任务：

- 自动捕获候选。
- promotion policy。
- deletion / tombstone / PII propagation。
- snapshot / restore。
- audit log。
- memory compaction / dreaming。
- provider migration。

验收：

- 自动记忆可审计。
- 用户可替换 promotion policy。
- provider 可迁移。

## 11. OpenSpec 建议

建议后续创建：

`openspec/changes/add-memory-fabric-runtime/`

建议 specs：

- `macaca-memory-fabric`
- `macaca-memory-provider-runtime`
- `macaca-memory-scope-policy`
- `macaca-memory-active-recall`
- `macaca-memory-knowledge-layer`

核心 requirements：

- 系统 SHALL 为每个 agent 提供独立的 AgentPrivate memory。
- 系统 SHALL 为每个 session/project 提供 SessionShared memory。
- 系统 SHALL 对所有 memory 操作强制携带 `MemoryScope`。
- 系统 SHALL 支持 agent private provider 与 session shared provider 独立配置。
- 系统 SHALL 支持 embedding provider 独立配置。
- 系统 SHALL 默认提供 Milvus vector memory backend。
- 系统 SHALL 将 `application_id` 映射为默认向量 database 隔离域。
- 系统 SHALL 将 `agent_id` / `agent_name` 映射为默认向量 collection 隔离单元。
- 系统 SHALL 允许用户替换 vector memory backend，但替代实现必须支持等价的 application database + agent collection 拓扑语义。
- 系统 SHALL 支持 active recall provider 独立配置。
- 系统 SHALL 支持 knowledge compiler 独立配置。
- 系统 SHALL 提供远程 memory provider 协议，允许用户替换默认记忆系统。
- 系统 SHALL 在 provider 失败时降级并产出 diagnostics，不终止 agent run。
- 系统 SHALL 记录 memory event provenance，供 trace UI 和 audit 使用。

## 12. 风险与缓解

| 风险 | 影响 | 缓解 |
| --- | --- | --- |
| Scope 设计过弱 | agent 私有记忆与项目共享记忆混淆 | 第一阶段强制 `MemoryScope`，测试覆盖跨 agent / 跨 session 隔离 |
| provider 边界过粗 | 无法自由装配 | capability 拆分，不让一个 provider 包办所有能力 |
| 自动捕获污染长期记忆 | recall 质量下降 | 候选层 + promotion policy + provenance |
| 外部 provider 慢 | 阻塞 agent run | timeout、circuit breaker、异步 sync、fallback |
| 配置能力强但用户难用 | 小白无法替换 | setup 只暴露 provider、endpoint、API key，高级配置可选 |
| 共享记忆泄漏私有信息 | 多 agent 互相污染 | visibility policy + explicit promotion |
| 记忆冲突不可控 | agent 使用过期事实 | freshness、supersedes、conflict group、wiki compiler |
| context engine 强耦合 provider | 后续替换困难 | context engine 只依赖 facade 和 report |

## 13. 最终建议

Macaca 应把记忆系统建设为一等基础设施。OpenClaw 的 memory-core、embedding provider、memory-wiki、active-memory、memory-lancedb 都应被纳入目标架构思考，只是以模块化方式吸收，而不是照搬其 TypeScript 实现。Hermes 的 provider 生命周期和 setup 体验应作为外部接入的易用性参考。

最关键的设计结论：

- 每个 agent 必须有自己的独家记忆。
- 每个 session/project 必须有共享记忆。
- 记忆 scope 必须是强类型主轴。
- provider 必须可插拔。
- capability 必须可组合。
- embedding、index、active recall、knowledge compiler 必须可独立替换。
- 小白用户必须能用远程 endpoint / MCP / API key 替换默认记忆系统。
- 上层 application 只能依赖 MemoryFacade，不依赖具体实现。

这套架构投入时间是值得的。记忆系统一旦设计好，会成为 Macaca 上层 application、context engineering、自我改进、多 agent 协作和长期自治的共同底座。
