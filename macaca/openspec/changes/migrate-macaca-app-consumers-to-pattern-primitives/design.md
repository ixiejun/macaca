# Design: 主要消费方迁移到基于设计模式重构后的 macaca-app 抽象

## Context

前一轮 `refactor-macaca-app-patterns` 已在 `macaca-app` 内部建立了新的 application-level 抽象：

- `AppRuntimeBuilder`
- `ApplicationRuntimeFactory`
- `WorkflowPromptParts`
- `WorkflowPromptStrategy`
- `AppCapabilitySet`

但如果 `macaca-web`、`macaca-task`、`macaca-framework`、`macaca-cli` 继续保留平行装配和平行解释逻辑，那么这些抽象仍然只是 crate 内部重构，而不是系统边界重构。

这一步的目标不是继续给 `macaca-app` 增加新概念，而是把“谁有权解释 application 语义”这件事真正收敛：

- application startup 由 `macaca-app` 抽象主导
- workflow prompt 由 `macaca-app` 抽象主导
- application-level capability / tool policy 由 `macaca-app` 抽象主导
- 主要消费方只做 runtime adapter / orchestration adapter / CLI adapter

## Goals

- 让主要消费方停止重复解释 application runtime / workflow prompt / tool policy。
- 将 application-level 语义稳定为 `macaca-app` 对外 contract。
- 让上层消费方以 builder / factory / strategy / composite 的结果为输入，而不是重新拼字符串和规则。
- 保持现有 trace、session、resume、todo、driver、MCP、skill 行为兼容。

## Non-Goals

- 不改变 manifest schema。
- 不重构 kernel 级协议。
- 不在本 change 中彻底消灭所有旧 helper 名称；允许 façade 继续存在，但内部必须委托并标记 `deprecated`。
- 不重写 planner/worker/coordinator 业务流程。

## Decisions

### 1. macaca-web 只保留 adapter 职责

`macaca-web` 是第一消费方。它后续应只负责：

- session / trace / SSE / EventLog adapter
- web runtime context 装配
- framework runner 的 runtime adapter

它不应继续：

- 自己定义 workflow prompt 解释规则
- 自己定义 application-level tool/driver policy 的语义
- 自己保留平行的 app runtime 装配逻辑

迁移方式：

- `FrameworkRunner` 继续保留 façade，但内部调用 `macaca-app` 提供的 prompt/runtime/capability 抽象。
- 与 application 相关的默认值、prompt 规则、tool policy 文本，必须从 `macaca-app` 结构化输入生成。

### 2. macaca-task 依赖 application contract，而不是 prompt 文本

`macaca-task` 需要知道 planner / worker 的 application 语义边界，但不应该自己推断：

- 哪个 workflow 是 entry workflow
- 哪些 capability / tool policy 属于应用层
- 哪些 prompt 规则来自 application 而非 task runtime

迁移方式：

- 引入由 `macaca-app` 暴露的稳定 contract 或 facade result。
- `macaca-task` 消费结构化结果，不再依赖调用侧把 application 规则翻译成字符串。

### 3. macaca-framework 接收结构化 application 输入

`macaca-framework` 不应只拿到最终拼好的 prompt 字符串。否则后续要支持更稳定的 trace、tool selection、driver policy，就只能再次在 framework 或上层复制规则。

迁移方式：

- framework-level construction / execution primitive 接收 `WorkflowPromptParts`、capability composite、tool policy 等结构化输入，或接收其稳定 facade。
- 最终 prompt 字符串仍可在兼容窗口保留，但不再作为唯一输入。

### 4. macaca-cli 统一到 app runtime factory

CLI 中 app startup / inspect / debug 路径如果绕开 `macaca-app` 的 builder/factory，会让 CLI 与 web/runtime 产生解释偏差。

迁移方式：

- CLI 统一通过 `AppRuntimeBuilder` / `ApplicationRuntimeFactory` 触发 app 装配或读取结果。
- CLI 只做命令行输入输出 adapter。

### 5. Deprecated Compatibility Path

对已经被结构化 contract 替代的旧 helper：

- `app_agent_base_prompt`
- `app_entry_agent_name_or`
- `legacy_app_task_planning_contract`

保留实现以便后续检索，但统一标记为 `deprecated`，并把仓库内 consumer 迁移到以下路径：

- entry agent fallback：直接使用 `app_entry_agent_name(...).unwrap_or(...)`
- prompt：使用 `app_agent_prompt_semantics(...).base_prompt`
- planning contract：使用 `app_task_planning_contract(...)` 或显式结构体构造

## Migration Plan

### Slice 1: Web 迁移

- 识别 `macaca-web` 中仍在重复解释 app runtime / workflow prompt / tool policy 的位置。
- 建立 `macaca-app` facade consumption path。
- 保留原 façade 名称，对外行为不变。
- 迁走对 `app_entry_agent_name_or` 等兼容 helper 的直接调用。

### Slice 2: Task 迁移

- 将 planner/worker 的 application-level 解释入口迁移到 `macaca-app` contract。
- 保持 task board / dependency / review / resume 语义不变。

### Slice 3: Framework 迁移

- 让 framework primitive 接收结构化 application 输入。
- 保持 traced construction 和 tool visibility 兼容。

### Slice 4: CLI 迁移

- 让 CLI 统一走 app runtime factory。
- 确保 inspect/debug/startup 输出与当前兼容。

## Risks / Trade-offs

- 风险：`macaca-web` 迁移时最容易影响实时 trace 和历史恢复  
  缓解：把 session/trace 验证列为强制检查，不允许只做 compile-level 通过。

- 风险：prompt 结构化后可能出现文本漂移，影响 planner/coordinator 行为  
  缓解：对 `FULLSTACK-AUTODEV` 与 `NEWSROOM-AUTOWRITER` 做 fixture 等价验证。

- 风险：给旧 helper 加 `deprecated` 后，仓库内仍有残留调用会扩大 warning 面  
  缓解：先迁 consumer，再标记 deprecated，并用 grep/编译验证仓库内无残留直接调用。

- 风险：framework 接入结构化 application 输入后，可能与先前 `macaca-agent` / `macaca-app` 提案重叠  
  缓解：本 change 明确聚焦“消费方迁移”，不重复设计 application 抽象本身。

## Verification

- `cargo test -p macaca-app`
- `cargo test -p macaca-web`
- `cargo test -p macaca-task`
- `cargo check -p macaca-framework -p macaca-web -p macaca-task -p macaca-cli`
- 至少一轮真实 session 验证：
  - 新建 session 无需刷新即可收到实时 trace
  - 刷新后可恢复历史 trace
  - trace 增量推送持续正常
- 每次改具体 symbol 前必须重新做 GitNexus impact
