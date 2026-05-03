# Change: 渐进式重构 macaca-runtime-host 宿主层抽象

## Why

`macaca-runtime-host` 是 Agent OS 宿主层里承接 MCP runtime、compat registry、环境桥接和资源回收的关键 crate。当前实现已经从 `macaca-web` 提升出来，但核心复杂度仍集中在 `mcp_runtime.rs`，一个模块同时负责 definition 构建、status probe、runtime key 计算、引用计数、tool 注册、cleanup 和兼容策略应用，导致职责边界模糊、后续扩展 transport / isolation / lease / factory 时改动面过大。

根据：

- `macaca/docs/design-pattern-refactor-plans/README.md`
- `macaca/docs/design-pattern-refactor-plans/refactor-order.md`
- `macaca/docs/design-pattern-refactor-plans/macaca-runtime-host.md`

本 change 需要把 `macaca-runtime-host` 的多个渐进式重构切片统一纳入一个方案中规划，再按小步骤实施，确保行为 1:1 保持，且为后续消费方迁移提供稳定、可检索的宿主层边界。

## What Changes

- 引入 `McpRuntimeFacade`，让宿主层消费者通过统一门面完成 definition 加载、status probe、tool 注册、cleanup。
- 引入 `McpTransport` bridge，把 transport 差异从宿主层 orchestration 中解耦。
- 引入 `McpSessionLease` 和资源回收命令模型，显式表达 session/app/agent 级 MCP 生命周期释放。
- 引入 `McpServerFactory` / `RuntimeEnvBuilder`，收口 server definition 组装、隔离参数、env 注入和 compat policy 应用。
- 保留旧 public 接口，但统一标记为 `deprecated`，禁止新增调用；旧接口必须委托到新实现，不得删除，便于后续迁移查找。
- 用单一 OpenSpec change 管理所有切片，但实施时必须按顺序逐步提交和验证，不能一次性大改。

## Non-Goals

- 不改变 MCP tool 的对外行为语义。
- 不改变现有 `McpRuntimeStatus`、tool policy、compat registry、env bridge 的业务含义。
- 不在本 change 中删除任何旧 public API。
- 不引入基于 app name、driver name 或具体业务名的专门逻辑。
- 不在未完成 proposal 审批前直接开始代码实现。

## Impact

- Affected specs: `macaca-runtime-host-core`
- Affected code:
  - `macaca/crates/macaca-runtime-host/src/lib.rs`
  - `macaca/crates/macaca-runtime-host/src/mcp_runtime.rs`
  - `macaca/crates/macaca-runtime-host/src/compat.rs`
  - `macaca/crates/macaca-runtime-host/src/env_bridge.rs`
  - 以及少量 `macaca-web` / 其他消费者的宿主层调用入口
- Expected risk: High
- Risk reason:
  - `macaca-runtime-host` 位于 MCP、skill-backed runtime、framework consumer 之间，是阶段 3 的宿主层收口点。
  - 生命周期与资源回收错误会直接影响 session cleanup、MCP tool 可见性和并发隔离。
  - 但本 change 采用“统一方案、分切片实现、旧接口委托保留”的策略，可以把风险拆散到多个可回滚提交中。
- GitNexus findings:
  - `cleanup_session` upstream risk 是 `CRITICAL`，直接调用者是 `post_chat_v2`，并命中 5 条 `post_chat_v2` 相关执行流；本次仅补测试覆盖，不再改动该生产控制流。
  - `probe_statuses` upstream risk 是 `LOW`，直接调用者是 `get_mcp_status`。
  - `register_tools` upstream risk 是 `LOW`，`McpRuntimeManager` upstream risk 也是 `LOW` / `impactedCount=0`。
  - 新增 symbol `McpRuntimeFacade`、`McpSessionLease`、`McpServerFactory`、`RuntimeEnvBuilder`、`McpTransport` 目前尚未被当前 GitNexus 索引命中；在下一次提交并重建索引前，blast radius 以对应 legacy entry points 作为记录基线。
- Behavioral compatibility:
  - MCP definition 解析、status probe、tool 注册、cleanup 的行为必须保持兼容。
  - 旧接口仅允许被标记为 `deprecated` 并委托到新实现，不得删除。
  - 新增 facade / bridge / lease / factory 后，对外观测到的 status、tool 名称、错误文本和清理时机不得无意变化。

## Rollout Strategy

本 change 统一规划全部切片，但必须按以下原则实施：

1. 先补 proposal / design / tasks / spec，锁定切片顺序和兼容要求。
2. 每个切片单独实现、单独编译、单独测试、单独回滚。
3. 新抽象先以 additive 方式引入，旧接口保留并委托。
4. 先迁 crate 内部调用，再迁消费方。
5. 全部消费方完成迁移前，旧接口保持存在并带 `deprecated` 标记。
