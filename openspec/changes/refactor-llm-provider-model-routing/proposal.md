# Change: Refactor LLM Provider and Model Routing into macaca-framework

## Why

当前 LLM provider / model 选择链路仍然分散且硬编码在 `macaca-web` 启动层：

- `macaca-web/src/lib.rs` 直接 `match default_provider` 构建 provider
- framework agent 统一只吃 `state.llm` 这一条单 provider 入口
- agent 侧虽然能指定 model 名，但 provider/model 解析、fallback、兼容 provider 注册都不在 framework 统一处理

这导致几个直接问题：

1. 新增 provider 需要改 server bootstrap，而不是注册配置
2. model 选择只是一段字符串，缺少统一的 provider/model resolution 语义
3. framework 与底层 LLM 层耦合不清，无法稳定支持 per-agent / per-task / per-request 路由
4. fallback 目前围绕“单 provider 默认模型”组织，跨 provider 扩展性差

如果后续要稳定支持更多 provider、更多模型、更多 agent 级覆盖，这条链路必须先收敛成统一的 registry + resolver + framework adapter。

## What Changes

- 在 `macaca-llm` 中引入统一的 provider registry / model routing 能力，按配置注册多个 provider
- 定义统一的 model target / model selection 语义，明确 provider、model、fallback 的解析顺序
- 在 `macaca-framework` 中接入基于 router 的 `ChatModel` 适配层，而不是只桥接单个 `LlmProvider`
- 让 framework agent 的 model 解析支持：
  - app 默认配置
  - agent 覆盖配置
  - 显式 provider:model 或 provider/model 引用
  - fallback chain
- 将 `macaca-web` 的 LLM 启动逻辑从“硬编码 provider 构造”改为“从 config 初始化 router / registry”
- 统一 coordinator / planner / worker 等 framework agent 的模型接入方式，后续新增 agent 不再自行拼接 provider/model

## Explicit Non-Goals

- 本提案不改前端 UI，不改 SSE / trace 协议
- 本提案不引入新的外部 provider SDK；仅重构接入方式与扩展点
- 本提案不修改 agent persona 或调度策略
- 本提案不在本轮统一解决 token streaming、cost accounting 展示等上层能力

## Impact

- Affected specs: `llm-provider-model-routing`
- Affected code:
  - `macaca/crates/macaca-llm`
  - `macaca/crates/macaca-framework`
  - `macaca/crates/macaca-web`
  - `macaca/crates/macaca-proto`
  - 配置加载与 app/framework agent 构建链路
- **BREAKING (internal)**:
  - `macaca-web` 不再直接持有“单 provider + 单默认模型”的假设
  - framework agent 构建入口将依赖统一的 model router / resolver
