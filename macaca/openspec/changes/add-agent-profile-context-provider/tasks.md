# Tasks

## 1. 审计与影响分析

- [x] 1.1 阅读 agent/application profile、persona、system prompt 的当前加载路径。
- [x] 1.2 阅读 context composer provider contract。
- [x] 1.3 对计划修改的 profile loader 和 prompt helper 符号运行 GitNexus upstream impact analysis。

## 2. Profile Model

- [x] 2.1 定义 `AgentProfileFileKind`。
- [x] 2.2 定义 `ProfileFileSnapshot`、`ProfileFileDiagnostics`（以 `ProfileLoadOutput` + provider diagnostics 落地）。
- [x] 2.3 定义 `ProfileFilePolicy`、priority、budget 和 target 策略（`ProfileKindPolicy` / `default_policy_for`）。
- [x] 2.4 定义 profile root/source 配置模型（`AgentProfileContextConfig`、`AgentProfileRootKind`）。

## 3. Provider Implementation

- [x] 3.1 实现 `ProfileFileContextProvider`。
- [x] 3.2 实现 realpath 越界检查、文件大小限制、编码/读取错误诊断。
- [x] 3.3 实现默认文件名和优先级。
- [x] 3.4 实现 `MEMORY.md` seed/audit 分类，不自动写入长期记忆。
- [x] 3.5 将 candidates 接入 composer/report。

## 4. Migration

- [x] 4.1 将现有 agent persona/system prompt profile 来源迁移为 provider candidates。
- [x] 4.2 标记被替代的 profile prompt helper deprecated。
- [x] 4.3 搜索并迁移生产代码中的 deprecated 调用。

## 5. Tests

- [x] 5.1 单测默认文件优先级。
- [x] 5.2 单测缺失文件不失败。
- [x] 5.3 单测 oversized/truncated diagnostics。
- [x] 5.4 单测 path escape 被拒绝。
- [x] 5.5 单测 `MEMORY.md` 不自动写入 vector memory。

## 6. Verification

- [x] 6.1 运行 `openspec validate add-agent-profile-context-provider --strict`。
- [x] 6.2 运行相关 Rust 测试。
- [x] 6.3 运行 `gitnexus_detect_changes()`。
