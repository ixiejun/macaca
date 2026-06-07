## 1. Preparation

- [x] 1.1 阅读 `macaca-context` 当前 `ContextEngine`、`ContextReport`、`PromptComposer`。
- [x] 1.2 阅读 `macaca-memory` facade/router/provider 相关模块。
- [x] 1.3 对计划修改的 context integration 和 memory facade 符号运行 GitNexus upstream impact analysis。

## 2. Active recall contract

- [x] 2.1 定义 `ActiveRecallCapability`。
- [x] 2.2 定义 `MemoryPrefetchRequest`、`MemoryPrefetchResult`。
- [x] 2.3 定义 `RecallCandidate`、`RecallDecision`、`ActiveRecallPolicy`。
- [x] 2.4 定义 budget DTO：max hits、max tokens/chars、latency timeout。

## 3. Default policy

- [x] 3.1 默认查询 `AgentPrivate`。
- [x] 3.2 默认查询 `SessionShared`。
- [x] 3.3 可配置查询 `ApplicationShared`、`UserScoped`、supplements。
- [x] 3.4 按 score/freshness/visibility/budget 合并。
- [x] 3.5 添加 skipped decision reason。

## 4. Context integration

- [x] 4.1 将 active recall 作为 `macaca-context` dynamic source。
- [x] 4.2 确保 recall injection 不写回 canonical transcript。
- [x] 4.3 `ContextReport` 记录 recall source breakdown。
- [x] 4.4 默认不保存完整 memory content。

## 5. Resilience

- [x] 5.1 provider recall timeout 后继续 fallback。
- [x] 5.2 provider error 记录 diagnostics。
- [x] 5.3 active recall 可通过配置禁用。

## 6. Tests

- [x] 6.1 agent private + session shared recall 合并测试。
- [x] 6.2 budget 截断测试。
- [x] 6.3 timeout fallback 测试。
- [x] 6.4 context report 不泄漏完整内容测试。
- [x] 6.5 dynamic/untrusted section 分类测试。

## 7. Verification

- [x] 7.1 运行 `cargo fmt`。
- [x] 7.2 运行 `cargo test -p macaca-memory -p macaca-context`。
- [x] 7.3 运行相关上层 cargo check。
- [x] 7.4 运行 `openspec validate add-memory-active-recall-integration --strict`。
- [x] 7.5 运行 `gitnexus_detect_changes()`。
