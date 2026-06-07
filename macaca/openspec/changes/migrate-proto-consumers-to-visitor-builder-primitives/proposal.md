# Change: 迁移上层 crate 到 macaca-proto 的 visitor / builder 原语

## Why

`refactor-macaca-proto-patterns` 已经把 `macaca-proto` 的第一批设计模式原语落地完成：

- `AgentExecutionEventVisitor`
- `AgentExecutionEvent::accept()`
- `MacacaConfigBuilder`
- `LlmProviderConfigBuilder`
- `ProtoErrorAdapter`

如果这些原语只停留在 `macaca-proto` 自身，而上层 crate 继续：

- 散落手写 `match AgentExecutionEvent`
- 大段手写 config struct 初始化
- 各自拼接错误展示文本

那么这次重构的收益就不会真正释放出来，项目仍然会继续在上层扩散重复逻辑。

本 change 的要求比上一轮更强，但实现口径必须服从“真实热点优先”：

- 所有上层消费 crate 必须立即迁移到本次基于设计模式的 visitor / builder / adapter 用法
- 但 `macaca-proto` 仍然只提供 additive-first 入口，不删除旧 API，不破坏旧 schema
- 如果某个 crate 经审计后不存在高频旧式消费路径，则该 crate 的“迁移完成”定义是：
  - 完成热点盘点
  - 证明不存在值得迁移的重复消费点
  - 约束后续新增代码默认优先使用 visitor / builder / adapter

也就是说：

- 兼容性保留在 proto 层
- 迁移责任落实到消费层

本 change 依据：

- `macaca/docs/design-pattern-refactor-plans/README.md` 的渐进式重构约束
- `macaca/docs/design-pattern-refactor-plans/refactor-order.md` 中“先稳底层 contract，再迁移主要消费方”的顺序要求
- 已完成的 `refactor-macaca-proto-patterns`

## What Changes

- 要求所有上层 crate 将 `AgentExecutionEvent` 的高频展示/转换/桥接逻辑迁移到 visitor 用法
- 要求所有上层 crate 将新写或重构中的高频 config 构造迁移到 builder 用法
- 要求所有上层 crate 将用户可见的 proto 错误展示迁移到 `ProtoErrorAdapter`
- 明确旧 enum `match`、旧大段 struct 初始化、旧错误展示拼接在上层 crate 中不再是推荐路径
- 保留 `macaca-proto` 中的旧 DTO/enum/构造方式，仅作为兼容层存在

## Scope

本 change 针对的“上层 crate”包括：

- `macaca-llm`
- `macaca-memory`
- `macaca-persist`
- `macaca-ipc`
- `macaca-gateway`
- `macaca-task`
- `macaca-tools`
- `macaca-driver`
- `macaca-skill`
- `macaca-runtime`
- `macaca-framework`
- `macaca-runtime-host`
- `macaca-kernel`
- `macaca-sdk`
- `macaca-app`
- `macaca-web`
- `macaca-cli`

不包括：

- `macaca-proto` 自身

## Non-Goals

- 不修改 `macaca-proto` 的既有字段、serde schema、event payload
- 不删除 `macaca-proto` 中的旧 API
- 不在本 change 中引入新的 proto contract
- 不借迁移机会改业务语义、trace schema、session schema、todo lifecycle
- 不把所有低价值、一次性的简单 `match` 都机械改写；只迁移高频、重复、职责性强的消费路径

## Impact

- Affected specs:
  - `macaca-proto-consumer-migration`
- Affected code:
  - `macaca/crates/**` 中所有直接消费 `AgentExecutionEvent`、proto config DTO、`MacacaError` 的上层 crate
- Expected risk: High
- Risk reason:
  - 这是跨多个 crate 的迁移型 change
  - 但它不改变底层 schema，只改变上层消费方式，因此风险集中在“迁移不完整”而不是“协议破坏”
- Compatibility:
  - wire format 不变
  - serde 不变
  - 事件名和 payload 不变
  - 旧 API 仍在 proto 层保留，但上层调用策略切换到新原语

## Rollout Strategy

本 change 必须分三层推进：

1. 先迁移最核心的高频消费方：
   - `macaca-web`
   - `macaca-framework`
   - `macaca-kernel`
2. 再迁移高频装配层：
   - `macaca-app`
   - `macaca-cli`
   - `macaca-runtime-host`
3. 最后补齐其余上层 crate 的一致性迁移和测试

但“完成定义”必须满足：

- 所有上层 crate 中高频 proto event 消费路径都已迁到 visitor
- 所有上层 crate 中高频 config 构造路径都已迁到 builder
- 所有上层 crate 中用户可见 proto 错误展示都已迁到 `ProtoErrorAdapter`
- 对不存在热点的 crate，必须有显式审计结论说明“无需机械迁移”
