# macaca-gateway 设计模式渐进式重构 Brainstorm 设计记录

## 背景

`macaca-gateway` 位于 `docs/design-pattern-refactor-plans/refactor-order.md` 的阶段 1，属于底层稳定 contract 中的“外围协议入口”。它当前只直接依赖 `macaca-proto`，主要被 `macaca-cli` 和 gateway integration tests 消费。

当前源码结构：

- `macaca/crates/macaca-gateway/src/adapter.rs`：定义 `ImAdapter` 和 `EventHandler`。
- `macaca/crates/macaca-gateway/src/gateway.rs`：管理 adapter 注册、启动、停止，并提供 `DefaultEventHandler`。
- `macaca/crates/macaca-gateway/src/telegram.rs`：Telegram 长轮询、消息解析、发送、格式拆分、测试集中在一个文件内。
- `macaca/crates/macaca-gateway/src/discord.rs`：Discord stub adapter。
- `macaca/crates/macaca-cli/src/commands.rs`：按配置手工创建 `Gateway`、Telegram/Discord adapter 并启动。

基线验证：

- `cargo test -p macaca-gateway -- --nocapture`：30 个测试通过。
- `cargo test -p macaca-integration-tests gateway -- --nocapture`：gateway 相关 integration tests 通过。

已发现的结构风险：

- `telegram.rs` 为 504 行，超过项目 500 行文件上限，且混合了 transport、parser、formatter、network loop、send strategy、测试。
- `ImAdapter` 目前既承担生命周期，又承担发送职责，还直接暴露 `GatewayEvent`，平台字段转换和内部事件语义耦合。
- `Gateway` 当前只是 adapter lifecycle manager，尚未具备 `GatewayMediator` 的 session routing / dispatch / reply 中介职责。
- `macaca-cli` 直接知道 Telegram/Discord concrete adapter，后续增加平台会继续扩大入口层 if/else。
- `TelegramAdapter::stop` 目前没有实际停止 background polling task，只是记录日志；本轮计划应先抽象生命周期，不改变运行语义。

## 设计模式适配

本轮只做渐进式、additive-first 重构，不接入真实 web session/chat_v2，不新增平台，不引入新依赖。旧接口标记 deprecated 后，CLI 生产启动路径迁移到 builder，避免继续直接调用 deprecated lifecycle API。

采用的设计模式：

- `Adapter`：平台原始消息转换为统一 `GatewayMessage` / `GatewayCommand` / `GatewayEvent`。
- `Mediator`：引入 `GatewayMediator` 中介角色，集中处理“收消息 -> 解析 -> 分发 -> 回复”的编排边界。
- `Observer`：为未来 Agent OS event stream 回写平台消息预留订阅接口，但本轮只建立接口和测试。
- `Strategy`：抽出平台回复格式化策略，先让 Telegram/Discord 复用现有 plain/html 行为，后续支持 Markdown/plain text 差异。
- `Factory/Builder`：用 `GatewayBuilder` 或 `GatewayFactory` 从配置创建 gateway，避免 CLI 继续手写具体平台分支。

## 可选方案

### 方案 A：最小 additive-first 抽象，保留现有接口

做法：

- 新增平台无关模型和 helper，但 `ImAdapter` / `Gateway` 旧接口继续可用。
- 为旧接口加 `#[deprecated]` 的时机放到后续“消费方迁移”提案，本轮只建立新入口。
- 拆分 `telegram.rs` 到 parser/formatter/client/lifecycle 边界，但保持 public re-export 不变。
- CLI 生产路径可在 builder 稳定后立即改用 `GatewayBuilder`，避免继续扩散 deprecated API。

优点：

- 风险最低，所有现有测试可 1:1 还原。
- 适合当前 gateway 仍是早期能力、未深度接入主 web runtime 的状态。
- 可逐步把 Telegram 超限文件拆开，不需要一次性重写 transport。

缺点：

- 短期会同时存在旧 `ImAdapter` 和新 `GatewayTransport`/`GatewayMediator`，API 面更宽。
- 需要同步更新 CLI 生产启动路径，变更面比纯 gateway crate 稍大。

### 方案 B：直接替换为 `GatewayTransport` + `GatewayMediator`

做法：

- 将 `ImAdapter` 替换为 `GatewayTransport`。
- `Gateway` 改造成 mediator，adapter 不再直接接收 `EventHandler`。
- CLI 立即改用新 factory 和 mediator。

优点：

- 结构最干净，抽象一次到位。
- 后续平台扩展成本最低。

缺点：

- 行为变更面大，容易破坏现有 Telegram polling 和 CLI 启动。
- 需要同时改 gateway、CLI、integration tests，违背“小切片、可逆、1:1 还原”的约束。

### 方案 C：先只拆 `telegram.rs`，暂不引入新抽象

做法：

- 只把 `parse_message`、`split_message`、send payload 等拆到子模块。
- 不引入 `GatewayTransport`、`GatewayMediator`、format strategy。

优点：

- 最小变更，能立即解决文件超限问题。

缺点：

- 没有推进设计模式重构目标。
- CLI concrete adapter if/else 和 gateway 编排职责混杂仍然存在。
- 后续仍要二次调整模块边界。

## 推荐方案

推荐采用方案 A：additive-first 抽象 + 小切片迁移。

理由：

- 符合项目“行为 1:1 还原”和“先抽象再替换”的约束。
- `macaca-gateway` 当前是底层外围入口，直接替换会向 `macaca-cli` 和 integration tests 扩散风险。
- 先建立平台无关模型、transport trait、mediator、formatter strategy 和 factory/builder，再把 CLI 生产启动路径切到 builder，能保持每一步可编译、可测试、可回滚。

## 渐进式设计

### 切片 1：平台无关消息模型与 Telegram parser adapter

新增 `GatewayInboundMessage` / `GatewayOutboundMessage` / `GatewayReply` 等 gateway 内部模型，先不修改 `macaca-proto::GatewayEvent`。Telegram 的 `parse_message` 迁移为平台 parser helper，输出仍可转换成现有 `GatewayEvent`，确保 tests 不变。

设计模式：Adapter。

风险：模型与 proto 已有 `GatewayMessage` 命名冲突。规避方式：gateway crate 内使用更明确的 `GatewayInboundMessage`，不修改 proto。

### 切片 2：GatewayTransport trait 与 legacy adapter bridge

新增 `GatewayTransport` trait，表达平台 transport 的最小能力：`name`、`start`、`send`、`stop`。保留 `ImAdapter`，并提供 bridge 或兼容实现，让现有 Telegram/Discord 可以同时满足旧接口和新接口。

设计模式：Bridge / Adapter。

风险：async trait object 和生命周期可能导致 trait 过宽。规避方式：保持方法签名接近旧接口，不引入泛型复杂度。

### 切片 3：GatewayMediator 编排边界

新增 `GatewayMediator`，负责接收规范化 inbound message，调用 `EventHandler` 或未来 session dispatcher，并生成 `GatewayReply`。本轮不接入 chat_v2，只保持当前 DefaultEventHandler 行为。

设计模式：Mediator。

风险：过早接入 session/chat_v2 会引入 web 依赖，破坏 gateway 只依赖 proto 的底层定位。规避方式：只定义 trait 边界，不引用 web/kernel。

### 切片 4：Reply formatting strategy

抽出 `GatewayReplyFormatter`，Telegram 默认 formatter 保持当前 HTML parse mode 和 4096 字符拆分行为；Discord stub formatter 只做 plain text passthrough。

设计模式：Strategy。

风险：格式化行为变化导致 Telegram 消息展示变化。规避方式：保留现有 `split_message` 测试，新增 formatter tests。

### 切片 5：Gateway factory/builder 与 CLI 生产迁移

新增 `GatewayBuilder` 或 `GatewayFactory`，根据 `GatewayConfig` 创建 gateway 和 adapters。旧生命周期 API 标记 deprecated 后，CLI 生产启动路径应迁移到 builder，兼容测试和内部 bridge 可保留局部 `#[allow(deprecated)]`。

设计模式：Factory / Builder / Facade。

风险：配置创建逻辑如果直接迁移 CLI，可能改变 disabled adapter 行为。规避方式：先建立 builder tests 覆盖 enabled/disabled combinations。

## 成功标准

- `macaca-gateway` 所有文件不超过 500 行。
- `cargo test -p macaca-gateway -- --nocapture` 通过。
- `cargo test -p macaca-integration-tests gateway -- --nocapture` 通过。
- 现有 `Gateway` / `ImAdapter` / `TelegramAdapter` / `DiscordAdapter` public API 在本轮保持兼容。
- CLI 生产路径不直接调用 deprecated gateway lifecycle API。
- 新增抽象不引用 `macaca-web`、`macaca-kernel` 或 application-specific 名称。
- 不引入新的第三方依赖。

## OpenSpec 提案边界

下一步 OpenSpec 应覆盖以下要求：

- gateway MUST expose platform-neutral inbound/outbound message primitives.
- gateway MUST keep legacy adapter behavior compatible during additive refactor.
- gateway SHOULD provide a mediator boundary without depending on web/kernel.
- gateway SHOULD provide platform formatting strategies.
- gateway SHOULD provide a config-driven builder/factory and CLI production startup should use it instead of deprecated lifecycle API.
