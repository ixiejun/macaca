# Change: 渐进式重构 macaca-proto 核心 contract

## Why

`macaca-proto` 处于 `refactor-order.md` 定义的阶段 1 起点，是整个 Agent OS 后端最底层、最广泛复用的 contract crate。它当前承担：

- agent / task / session / event DTO
- config DTO
- error DTO
- orchestration event 枚举

这些对象应该稳定、纯粹、低策略，但目前仍存在几个渐进重构价值：

- 事件枚举的展示、转换、持久化逻辑容易在各 crate 中散落成重复 `match`
- 部分 config DTO 构造成本高，测试与调用侧会反复手写默认字段
- 错误展示和适配仍可能在上层各自处理，用户可见信息一致性不够强
- DTO 与运行时策略的边界需要进一步固定，避免后续把 framework/kernel/task 的行为继续塞回 proto

本 change 依据：

- `macaca/docs/design-pattern-refactor-plans/README.md` 的全局渐进式重构约束
- `macaca/docs/design-pattern-refactor-plans/refactor-order.md` 中“阶段 1：最底层稳定 contract”的顺序要求
- `macaca/docs/design-pattern-refactor-plans/macaca-proto.md` 的 crate 级渐进式重构计划

目标是在完全保持 wire schema、序列化兼容和上层行为不变的前提下，把 `macaca-proto` 收敛成更稳定的 contract 层。

## What Changes

- 为核心 event enum 增加 visitor 风格访问接口，先让行为从散落 `match` 向显式访问协议收敛。
- 为高频 config DTO 增加 builder 路径，旧 struct 初始化方式继续保留。
- 为 proto 层错误增加统一的 display/code 适配入口，供上层按一致规则展示。
- 在 spec 中明确：`macaca-proto` 只承载数据 contract，不承载 runtime strategy、planner policy、tool policy、resume policy。

## Non-Goals

- 不修改任何现有 DTO 的字段名、序列化字段、事件名、payload schema。
- 不把运行时状态机、planner/decomposer/reviewer 策略移动到 `macaca-proto`。
- 不在本 change 中重构 `macaca-persist`、`macaca-task`、`macaca-framework`、`macaca-web` 的消费逻辑。
- 不要求所有上层 crate 立即迁移到 visitor / builder，只提供 additive-first 入口。
- 不引入 application-specific 或 driver-specific DTO 语义。

## Impact

- Affected specs: `macaca-proto-core`
- Affected code:
  - `macaca/crates/macaca-proto/src/**`
  - 可能涉及少量依赖 `macaca-proto` 的单元测试或调用侧编译适配，但不应改变业务语义
- Expected risk: Medium
- Risk reason:
  - `macaca-proto` 被几乎所有核心 crate 依赖，任何破坏性字段变化都会放大影响
  - 但本 change 采用 additive-first 策略：新增 visitor / builder / adapter，不删除旧 DTO 路径
- Behavioral compatibility:
  - 所有 DTO 的 serde 行为必须保持不变
  - 所有 event enum 的名称、字段和值必须保持不变
  - 所有 config builder 输出必须与手写 struct 构造等价
  - 所有错误展示适配必须保持旧错误含义不变，仅统一展示入口

## Rollout Strategy

本 change 必须按小切片推进：

1. 先锁定 `macaca-proto` 当前序列化、默认值和错误展示行为
2. 再增加 event visitor 接口，不改 enum 字段
3. 再增加 config builder，仅让新代码和测试先使用
4. 再补统一错误适配入口，但保留旧错误类型与旧调用方式
5. 最后在 spec 与文档中明确 proto 与 runtime strategy 的职责边界

每个切片都必须：

- 独立编译
- 独立测试
- 可单独回滚

