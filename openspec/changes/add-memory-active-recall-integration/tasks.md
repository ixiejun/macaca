## 1. Preparation

- [ ] 1.1 阅读 `macaca-context` 当前 `ContextEngine`、`ContextReport`、`PromptComposer`。
- [ ] 1.2 阅读 `macaca-memory` facade/router/provider 相关模块。
- [ ] 1.3 对计划修改的 context integration 和 memory facade 符号运行 GitNexus upstream impact analysis。

## 2. Active recall contract

- [ ] 2.1 定义 `ActiveRecallCapability`。
- [ ] 2.2 定义 `MemoryPrefetchRequest`、`MemoryPrefetchResult`。
- [ ] 2.3 定义 `RecallCandidate`、`RecallDecision`、`ActiveRecallPolicy`。
- [ ] 2.4 定义 budget DTO：max hits、max tokens/chars、latency timeout。

## 3. Default policy

- [ ] 3.1 默认查询 `AgentPrivate`。
- [ ] 3.2 默认查询 `SessionShared`。
- [ ] 3.3 可配置查询 `ApplicationShared`、`UserScoped`、supplements。
- [ ] 3.4 按 score/freshness/visibility/budget 合并。
- [ ] 3.5 添加 skipped decision reason。

## 4. Context integration

- [ ] 4.1 将 active recall 作为 `macaca-context` dynamic source。
- [ ] 4.2 确保 recall injection 不写回 canonical transcript。
- [ ] 4.3 `ContextReport` 记录 recall source breakdown。
- [ ] 4.4 默认不保存完整 memory content。

## 5. Resilience

- [ ] 5.1 provider recall timeout 后继续 fallback。
- [ ] 5.2 provider error 记录 diagnostics。
- [ ] 5.3 active recall 可通过配置禁用。

## 6. Tests

- [ ] 6.1 agent private + session shared recall 合并测试。
- [ ] 6.2 budget 截断测试。
- [ ] 6.3 timeout fallback 测试。
- [ ] 6.4 context report 不泄漏完整内容测试。
- [ ] 6.5 dynamic/untrusted section 分类测试。

## 7. Verification

- [ ] 7.1 运行 `cargo fmt`。
- [ ] 7.2 运行 `cargo test -p macaca-memory -p macaca-context`。
- [ ] 7.3 运行相关上层 cargo check。
- [ ] 7.4 运行 `openspec validate add-memory-active-recall-integration --strict`。
- [ ] 7.5 运行 `gitnexus_detect_changes()`。
