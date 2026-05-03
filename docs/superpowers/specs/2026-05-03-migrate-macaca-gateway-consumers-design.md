# 迁移 macaca-gateway 上层消费方 Brainstorm 设计记录

## 背景

本轮目标不是继续扩展 `macaca-gateway` 自身抽象，而是把上层代码迁移到上一轮基于设计模式重构后的 gateway primitives。

当前磁盘上的 `macaca-gateway` 重构已经引入：

- `GatewayBuilder`：配置驱动构建 gateway，避免上层手写 Telegram/Discord 分支。
- `GatewayTransport`：平台 transport 边界，作为旧 `ImAdapter` 的替代方向。
- `GatewayMediator` / `GatewayEventSink`：中介者边界，隔离 gateway 入站消息与事件分发。
- `GatewayReplyFormatter`：回复格式化 strategy。
- `GatewayInboundMessage` / `GatewayReply`：平台无关消息模型。
- 旧 `ImAdapter`、`EventHandler`、`Gateway::new/register_adapter/start_all/stop_all` 已标记 deprecated，但保留兼容。

Cargo 依赖扫描结果：

- 生产消费方只有 `macaca-cli` 直接依赖 `macaca-gateway`。
- 测试消费方只有 `macaca-integration-tests` 直接依赖 `macaca-gateway`。
- `macaca-web` 当前没有直接依赖 `macaca-gateway`，不应在本轮引入反向耦合。

源码扫描结果：

- `macaca/crates/macaca-cli/src/commands.rs` 当前已经使用 `GatewayBuilder::new(config.gateway.clone()).start().await?`。
- `macaca/crates/macaca-integration-tests/tests/gateway_pipeline.rs` 仍然使用 `Gateway`、`ImAdapter`、`EventHandler`、`TelegramAdapter`、`DiscordAdapter` 的 legacy lifecycle API。
- gateway crate 内部的 `builder.rs` 仍通过旧 `Gateway` 做兼容 bridge，这是本轮允许的内部兼容层，不属于上层消费方问题。

GitNexus 注意事项：

- 当前 GitNexus 图对 `run_kernel` 的 outgoing refs 仍显示旧的 `Gateway::register_adapter/start_all`，说明索引未覆盖当前未提交重构改动。
- 本轮判断以磁盘源码和 Cargo 依赖为准；后续提交后需要重建 GitNexus 索引。

## 设计模式适配

本轮不新增设计模式，只迁移消费方到已存在的模式边界：

- **Builder / Factory**：CLI 和新 integration coverage 使用 `GatewayBuilder`，不再手写 concrete adapter registration。
- **Mediator**：新增或调整测试覆盖 `GatewayMediator` / `GatewayEventSink` 消费方式，避免只测试旧 `EventHandler`。
- **Strategy**：formatter 已由 gateway crate 内部测试覆盖，上层不需要直接依赖具体 strategy。
- **Bridge / Adapter**：legacy `ImAdapter` 保留在 gateway crate 内部和兼容测试中，生产上层不直接调用。

## 可选方案

### 方案 A：只确认 CLI 已迁移，不改 integration tests

做法：

- 保持 `macaca-cli` 使用 `GatewayBuilder`。
- `macaca-integration-tests` 继续全部走 legacy API。
- 只增加 grep 验证，确认生产上层没有 deprecated gateway 调用。

优点：

- 变更最少。
- 风险最低。

缺点：

- integration tests 没有覆盖新 builder/mediator 消费路径。
- 以后 `GatewayBuilder` 可能只被 CLI 启动路径间接覆盖，缺少独立集成级保护。
- “迁移上层代码”语义不完整，因为测试上层仍只体现旧接口。

结论：不推荐作为最终方案，只适合作为现状确认。

### 方案 B：生产路径迁移 + integration tests 双轨覆盖

做法：

- 保持 `macaca-cli` 使用 `GatewayBuilder`。
- 在 `macaca-integration-tests` 中新增 builder-based lifecycle 测试，覆盖 enabled/disabled adapters 的上层构建路径。
- 在 `macaca-integration-tests` 中新增 mediator/event sink 测试，覆盖 `GatewayMediator` 的上层消费方式。
- 保留少量 legacy integration tests，并显式命名为 compatibility coverage，局部 `#![allow(deprecated)]` 或模块级 `#[allow(deprecated)]`。
- 验证生产上层中不存在 `Gateway::new/register_adapter/start_all/stop_all`、`ImAdapter`、`EventHandler` 的直接调用。

优点：

- 满足 additive-first 和 1:1 行为还原。
- 生产代码不再依赖 deprecated API。
- 测试层同时保护新入口和旧兼容面，后续迁移风险低。
- 不需要引入 `macaca-web` 或 application-specific 逻辑。

缺点：

- integration tests 会短期保留新旧两套覆盖。
- deprecated grep 需要区分“生产上层禁止”和“兼容测试允许”。

结论：推荐。

### 方案 C：彻底迁移所有测试，删除 legacy integration coverage

做法：

- integration tests 全部改成 `GatewayBuilder` / `GatewayMediator`。
- 不再在上层测试旧 `Gateway` / `ImAdapter` / `EventHandler`。

优点：

- 上层调用面最干净。
- deprecated API 只留在 gateway crate 内部。

缺点：

- 旧 API 仍承诺保留但缺少跨 crate 兼容测试。
- 如果外部用户仍使用旧 API，破坏兼容时不容易被发现。
- 与“标记 deprecated 但不要删除，便于后续迁移查找”的项目策略不一致。

结论：不推荐。

## 推荐方案

采用方案 B：生产路径迁移 + integration tests 双轨覆盖。

本轮的核心边界：

- `macaca-cli` 是唯一生产消费方，必须使用 `GatewayBuilder`。
- `macaca-integration-tests` 应新增新入口覆盖，但允许保留 legacy compatibility coverage。
- `macaca-gateway` 内部 bridge 可以继续使用 deprecated API，并通过局部 `#[allow(deprecated)]` 明确标注。
- 不把 gateway 接入 `macaca-web` / `chat_v2` / session / app runtime；这些属于后续能力，不属于消费方迁移。

## 风险与控制

- 风险：误把 compatibility tests 也强制迁移，导致旧 API 兼容性失去跨 crate 覆盖。
  控制：测试命名中明确 `legacy_*_compatibility`，并只在这些测试中允许 deprecated。

- 风险：CLI 启动路径是主入口，任何改动都可能影响 `macaca run`。
  控制：本轮不扩大 CLI 行为，只保留 `GatewayBuilder` 启动路径，并跑 `cargo check -p macaca-cli`。

- 风险：`GatewayBuilder` 当前仍内部桥接到旧 `Gateway`，容易被误判为迁移不彻底。
  控制：本轮关注上层消费边界；内部 bridge 是 additive-first 兼容策略，真正替换内部 lifecycle manager 应另开 gateway crate 后续提案。

- 风险：GitNexus 索引陈旧导致 impact 信息不准确。
  控制：提交后重建 GitNexus；实施前如果继续编辑 `run_kernel` 或 integration test symbols，按规则重新跑 impact。

## 成功标准

- 生产上层代码中没有直接调用 deprecated gateway lifecycle API。
- `macaca-cli` 继续通过 `GatewayBuilder` 启动 gateway。
- integration tests 覆盖 `GatewayBuilder` 消费路径。
- integration tests 覆盖 `GatewayMediator` / `GatewayEventSink` 消费路径。
- legacy gateway API 只在 gateway crate 内部兼容桥、gateway crate 单测、明确命名的 compatibility integration tests 中使用。
- `cargo test -p macaca-gateway -- --nocapture` 通过。
- `cargo test -p macaca-integration-tests gateway -- --nocapture` 通过。
- `cargo check -p macaca-gateway -p macaca-cli -p macaca-integration-tests` 通过。
- 后续提交后重建 GitNexus 索引。
