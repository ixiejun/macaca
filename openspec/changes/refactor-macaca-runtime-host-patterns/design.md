# Design: macaca-runtime-host 渐进式宿主层重构

## Context

`macaca-runtime-host` 当前是 Agent OS 宿主侧 MCP glue 的集中层，负责：

- MCP registry / runtime manager
- compat registry
- env bridge
- lifecycle scope / runtime key / cleanup
- tool registration

当前实现的优点是功能已经集中到单一 crate，但主要复杂度仍集中在 [mcp_runtime.rs](file:///Users/quantum/Code/dev/agent/macaca/crates/macaca-runtime-host/src/mcp_runtime.rs)。这会带来两个问题：

1. transport、lifecycle、resource cleanup、definition 构建没有清晰边界；
2. 后续 `macaca-web`、未来 `cli` / `gateway` 等消费者迁移时，仍可能继续依赖内部细节而不是稳定宿主门面。

本设计遵循项目约束：

- 小切片、可回滚、行为 1:1 保持
- 不引入 app-specific 逻辑
- 不删除旧接口，旧接口统一 `deprecated` 并委托
- 先抽象、再替换、再迁移消费者

## Goals

- 为 `macaca-runtime-host` 建立稳定的宿主层 facade。
- 分离 transport、lease、factory、env 组装职责。
- 让 MCP 资源获取与释放具备显式的 session/app/agent 生命周期语义。
- 为消费方迁移提供可搜索的 deprecated 旧接口。
- 保持现有 MCP status、tool registration、cleanup 行为兼容。

## Non-Goals

- 不重写 `macaca-framework::mcp` 协议实现。
- 不更改现有 MCP tool 暴露语义或命名策略。
- 不在本轮删除 `McpRuntimeManager`、`definitions_from_skill_snapshot` 等旧 public API。
- 不把本 crate 改造成只服务 `macaca-web` 的专用实现。

## Design Principles

- `Facade` 优先：先统一宿主层调用入口，再拆内部职责。
- `Bridge` 渐进：transport 抽象先作为内部协议，不强迫一步到位替换全部调用点。
- `Command + Memento` 渐进：lease 释放与资源回收先显式建模，再接入完整恢复链路。
- `Builder` 只收口组装责任，不改变对外配置语义。
- `Deprecated but delegated`：旧接口只做兼容桥，不允许继续承载新增逻辑。

## Proposed Design

### 1. McpRuntimeFacade

第一切片引入统一宿主层门面：

```rust
pub struct McpRuntimeFacade {
    manager: Arc<McpRuntimeManager>,
}

impl McpRuntimeFacade {
    pub async fn probe(&self, policy: &McpToolPolicy) -> Vec<McpRuntimeStatus>;
    pub async fn register(
        &self,
        toolkit: &mut Toolkit,
        policy: &McpToolPolicy,
        context: &McpRuntimeContext,
    ) -> Vec<McpRuntimeStatus>;
    pub async fn register_definitions(
        &self,
        toolkit: &mut Toolkit,
        definitions: Vec<McpServerDefinition>,
        policy: &McpToolPolicy,
        context: &McpRuntimeContext,
    ) -> Vec<McpRuntimeStatus>;
    pub async fn cleanup_session(&self, session_id: &str) -> Vec<McpRuntimeStatus>;
    pub async fn cleanup_app(&self, app_id: &ApplicationId) -> Vec<McpRuntimeStatus>;
}
```

规则：

- `macaca-web` 等消费方优先依赖 facade。
- `McpRuntimeManager` 暂时保留，但不再作为推荐新入口。
- facade 只负责宿主层操作编排，不复写底层协议实现。

### 2. McpTransport Bridge

第二切片把 transport 差异从宿主层流程中拆开：

```rust
pub trait McpTransport: Send + Sync {
    fn config(&self) -> &McpTransportConfig;
    fn label(&self) -> &'static str;
    fn create_client(&self, timeouts: McpTimeouts) -> Result<Box<dyn McpClient>, McpRuntimeError>;
}
```

目标：

- 把 `stdio` / `sse` / `streamable_http` 的 client 创建逻辑从 `match self.transport` 中拆走。
- `McpServerDefinition` 仍保留兼容字段，不强制改变序列化结构。
- 现有 `client_from_transport(...)` 逻辑通过 adapter 进入 bridge。

### 3. McpSessionLease

第三切片显式表达资源租约与释放：

```rust
pub struct McpSessionLease {
    key: McpRuntimeKey,
    cleanup: Vec<ResourceCleanupCommand>,
}

impl McpSessionLease {
    pub async fn release(self, facade: &McpRuntimeFacade) -> Result<Option<McpRuntimeStatus>, McpRuntimeError>;
}
```

目标：

- 把 `acquire_runtime_key` / `release_runtime_key` 从裸引用计数升级为显式 lease 语义。
- close callback、task completion、timeout cleanup 统一落到 lease release。
- 为 Playwright / browser profile / stateful MCP 等资源回收问题建立固定释放点。

### 4. McpServerFactory + RuntimeEnvBuilder

第四切片把组装逻辑从 manager 中收口：

```rust
pub struct RuntimeEnvBuilder { ... }
pub struct McpServerFactory { ... }
```

职责：

- 组装 runtime env
- 应用 compat concurrency isolation
- 生成 stdio/http/sse transport
- 组装 `McpServerDefinition`
- 向后兼容现有 YAML / skill snapshot / compat registry 输入

这样可以把当前散落在：

- `McpServerConfigEntry::into_definition`
- `definitions_from_skill_snapshot_with_registry`
- `compat.rs`
- `env_bridge.rs`

之间的组装逻辑收敛到固定边界。

### 5. Deprecated Compatibility Layer

旧 public API 保留，但必须统一遵守以下规则：

- 添加 `#[deprecated(note = "...")]`
- 文档明确迁移目标
- 内部只做委托，不再新增逻辑
- 新代码禁止继续直接依赖这些旧入口

初步目标对象包括：

- `McpRuntimeManager` 的直接消费入口
- 旧的 free function 风格注册/definition helper
- 可能仍暴露给消费方的直接 manager 操作路径

## Migration Strategy

### Slice 1

- 引入 `McpRuntimeFacade`
- `macaca-runtime-host` 内部适配现有 manager
- 消费方优先切到 facade

### Slice 2

- 引入 `McpTransport` bridge
- 用 adapter 封装现有 transport client 创建
- 保持 `McpServerDefinition` schema 不变

### Slice 3

- 引入 `McpSessionLease`
- 让 runtime key 获取/释放通过 lease 表达
- 调整 close callback / cleanup 路径委托到 lease

### Slice 4

- 引入 `McpServerFactory` / `RuntimeEnvBuilder`
- 统一 definition 组装和 env / isolation 注入

### Slice 5

- 标记旧接口 deprecated
- 迁移 crate 内与主要消费方调用点
- 保留旧路径供后续全仓迁移检索

## Risks / Trade-offs

- 风险：facade 只包一层，早期内部复杂度仍然存在
  - 缓解：在 design 和 tasks 里明确后续 bridge / lease / factory 切片，不让 facade 变成终点
- 风险：lease 改造触碰 close callback 和 cleanup 语义
  - 缓解：先锁定现有测试，再引入显式 lease
- 风险：deprecated 旧接口长期共存导致双轨维护
  - 缓解：旧接口禁止新逻辑进入，只允许委托
- 风险：消费者迁移不彻底，导致 facade 没有真正形成边界
  - 缓解：把“新代码禁止调用 deprecated 接口”写入规范与 tasks

## Compatibility Rules

- 旧 public API 不删除，只能 `deprecated` 并委托。
- 现有 status API 结构不变。
- 现有 tool registration 冲突策略不变。
- 现有 compat registry 匹配语义不变。
- 现有 env bridge placeholder / env forwarding 语义不变。
- 任何资源 cleanup 行为变化都必须先有测试锁定。

## Verification

- `cargo test -p macaca-runtime-host`
- `cargo check -p macaca-runtime-host`
- 如消费方迁移受影响，再运行：
  - `cargo check -p macaca-web`
  - 必要时补充其他直接消费者
- 实施前必须按仓库规则对将修改的 symbol 逐一做 GitNexus impact analysis。
- proposal 完成后运行 `openspec validate refactor-macaca-runtime-host-patterns --strict`。
