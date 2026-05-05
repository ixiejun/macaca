# Tasks

## 1. 审计与影响分析

- [x] 1.1 阅读 `macaca-framework`、`macaca-runtime`、`macaca-web`、`macaca-agent`、`macaca-sdk` 中所有 LLM 请求入口。
- [x] 1.2 阅读当前 prompt 来源：persona、application prompt semantics、capabilities、workspace paths、skill snapshot、memory、tool schema、session history、trace/tool result。
- [x] 1.3 对计划修改的核心符号运行 GitNexus upstream impact analysis，并记录直接调用者、受影响流程和风险等级。
- [x] 1.4 确认没有现有 OpenSpec change 与 `context-engine` 能力冲突。

## 2. 新增 context contract

- [x] 2.1 新增 `macaca-context` crate，并接入 workspace。
- [x] 2.2 定义 `ContextEngine`、`ContextEngineInfo`、`ContextAssembleInput`、`ContextAssembleResult`、`ContextAfterTurnInput`。
- [x] 2.3 定义 `ContextBudget`、`ContextReport`、`ContextSourceReport`、`ContextDecisionReport`。
- [x] 2.4 定义 `PromptSection`、`PromptStability`、`TrustLevel`、`ContextSourceKind`、`PromptComposer`。
- [x] 2.5 增加 `LegacyContextEngine`，默认返回与输入等价的 messages/options，并生成基础 report。
- [x] 2.6 为 context contract 添加单元测试，覆盖 legacy 等价、report 字段、stable/dynamic 分类。

## 3. 接入 framework/runtime 请求路径

- [x] 3.1 在 `macaca-framework` 的 ReAct 模型调用前接入 context facade 或 legacy engine，不改变实际发送 messages/options。
- [x] 3.2 在 `macaca-runtime` 的 direct agentic loop 模型调用前接入 context facade 或 legacy engine，不改变现有 `ContextWindowManager` 行为。
- [x] 3.3 检查 `macaca-agent` 和 `macaca-sdk` 简单/declarative agent 路径，确保可以保持 legacy 兼容或明确记录后续迁移任务。
- [x] 3.4 为 framework/runtime legacy 接入增加等价测试或 snapshot 测试。

## 4. PromptComposer 分层

- [x] 4.1 将现有 system prompt 来源映射为 typed prompt sections。
- [x] 4.2 区分 stable 与 dynamic sections，并添加 explicit boundary。
- [x] 4.3 对 skill、capability、tool、workspace 等列表进行确定性排序。
- [x] 4.4 计算 `stable_prompt_hash` 和 `prompt_hash`。
- [x] 4.5 增加测试验证动态信息变化不影响 stable hash。

## 5. ContextReport 持久化和读取

- [x] 5.1 选择首轮 report summary 存储位置，优先复用现有 session/event/report 可查询路径。
- [x] 5.2 在每次模型请求后保存 report summary，不默认保存完整 prompt。
- [x] 5.3 增加后端 API 或 facade 读取 session/agent/request 的 context report。
- [x] 5.4 在 Web/trace UI 中提供最小 report summary 入口，展示 engine id、token breakdown、source breakdown、hash、warning。
- [x] 5.5 为 API/facade 增加测试，确认不会泄漏完整 prompt。

## 6. 上层迁移与 deprecation

- [x] 6.1 将新代码路径迁移到 `ContextManagerFacade` 或 `ContextEngine` 抽象。
- [x] 6.2 将被替代的直接 prompt/context 构造入口标记为 deprecated，并在说明中要求使用 context facade。
- [x] 6.3 使用 `rg` 检查生产代码中不再新增 deprecated 调用。
- [x] 6.4 保留 deprecated 接口，不删除，便于后续迁移和查找。

## 7. 验证

- [x] 7.1 运行 `openspec validate add-pluggable-context-engine-foundation --strict`。
- [x] 7.2 运行相关 Rust 单元测试和集成测试。
- [x] 7.3 运行前端/API 相关测试或手动验证 report UI 不影响现有会话加载。
- [x] 7.4 运行 `gitnexus_detect_changes()`，确认影响范围符合预期。
- [x] 7.5 更新本任务清单状态，确保代码与 OpenSpec 对齐后再归档。
