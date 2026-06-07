# Design: 上层 crate 迁移到 macaca-proto visitor / builder 原语

## Context

`macaca-proto` 已完成第一轮 additive-first 重构，但按照 `refactor-order.md` 的顺序，底层 contract 稳定之后，下一步就应该迁移主要消费方，而不是让旧消费方式继续长期并存。

这次 design 的核心立场是：

- 兼容性留在 proto 层
- 上层 crate 立即收敛到新的设计模式原语

这样才能确保：

- event 处理逻辑不再到处散落 `match`
- config 构造不再到处展开默认字段
- 错误展示不再各写一套

## Goals

- 让上层 crate 对 `AgentExecutionEvent` 的高频消费统一走 visitor
- 让上层 crate 对高频 proto config DTO 的构造统一走 builder
- 让上层 crate 的用户可见 proto 错误展示统一走 adapter
- 不改变任何 wire schema 和业务语义

## Non-Goals

- 不删除 `macaca-proto` 中旧的 enum/struct 初始化方式
- 不迁移所有低价值、单点、一次性的简单消费代码
- 不借机改变 task/trace/session 行为
- 不在本 change 中新增更多 proto 原语

## Migration Principles

### 1. Visitor 迁移原则

适合迁移到 visitor 的代码必须满足以下至少一条：

- 同一 event enum 被多次 `match`
- 同一个 `match` 同时承担展示、转换、桥接、持久化职责
- 同一逻辑在多个 crate 中重复出现

优先迁移场景：

- SSE 转换
- trace step 聚合
- event log 展示
- runtime bridge
- driver / framework / web 的事件转译

不必强制迁移的场景：

- 单个测试里的一次性断言
- 极短的、只判断一个 variant 的局部逻辑
- 仅负责创建或透传 `AgentExecutionEvent`，而不承担重复展示/转换职责的桥接代码

### 2. Builder 迁移原则

builder 迁移目标不是“把所有 struct 初始化都禁掉”，而是优先收敛：

- 测试里重复手写大量默认字段
- 入口装配层只改 1-2 个字段却要展开整个 config
- 同一个 config 在多个 crate 中以近似方式反复手写

优先迁移场景：

- `macaca-app`
- `macaca-cli`
- `macaca-runtime-host`
- `macaca-web` 启动/测试装配

如果审计后确认某个上层 crate 没有真实的重复 `MacacaConfig` / `LlmProviderConfig`
手写构造点，则不为了“完成迁移”去人为制造 builder 用法。此时的完成标准是：

- 盘点结果落入 checklist
- 保持新代码默认优先使用 builder
- 等真实热点出现时再迁移

### 3. Error Adapter 迁移原则

凡是“用户能看到”的 proto 错误展示，都应切到 `ProtoErrorAdapter`。

典型场景：

- API 返回错误文本
- CLI 输出错误
- trace / SSE / event log 错误展示
- runtime host 或 framework 的桥接错误输出

但 adapter 不接管：

- HTTP status
- retry 策略
- recovery 策略

## Proposed Design

### Layer A: 核心高频消费方

先迁移：

- `macaca-web`
- `macaca-framework`
- `macaca-kernel`

原因：

- 这些 crate 承担最多的事件展示、trace 转换、桥接逻辑
- 迁移收益最大
- 也最容易形成新的统一用法样板

审计结论允许出现两类结果：

- 有真实高频消费点：必须落代码迁移
- 没有真实高频消费点：以审计结论收口，不做机械改写

### Layer B: 装配与入口层

再迁移：

- `macaca-app`
- `macaca-cli`
- `macaca-runtime-host`

重点：

- builder
- error adapter
- 少量 event visitor 的展示逻辑

### Layer C: 其余上层 crate

最后收口：

- `macaca-task`
- `macaca-tools`
- `macaca-driver`
- `macaca-skill`
- `macaca-runtime`
- `macaca-llm`
- `macaca-memory`
- `macaca-persist`
- `macaca-ipc`
- `macaca-gateway`
- `macaca-sdk`

这些 crate 应以“消除重复消费模式”为目标，不做机械改写。

## Compatibility Rules

- 所有迁移都必须保持旧输出行为一致
- visitor 迁移后，event name / payload / trace schema 不得变化
- builder 迁移后，生成 config 必须与原手写构造等价
- error adapter 迁移后，用户可见错误含义不得变化
- 旧 API 仍保留在 proto 层，但新代码和迁移后的消费层不再优先使用旧路径

## Audit-Based Completion

本 change 允许“审计即完成”的子项，但前提必须同时满足：

- 已完成全文搜索 / GitNexus / 调用链盘点
- 证明该 crate 当前不存在重复、高频、职责性强的旧式消费路径
- 记录为何不迁移，避免后续误以为该项被遗漏

## Verification

- 分 crate 运行测试和 `cargo check`
- 至少运行：
  - `cargo check -p macaca-web -p macaca-framework -p macaca-kernel`
  - `cargo check -p macaca-app -p macaca-cli -p macaca-runtime-host`
- 运行 workspace `cargo check`
- 对高风险符号逐个做 GitNexus impact
- 最终运行 `gitnexus_detect_changes(scope: "all")`
