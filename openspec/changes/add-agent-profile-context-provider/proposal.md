# Change: 增加 Agent Profile Context Provider

## Why

Macaca 的每个 agent 需要通过 `AGENTS.md`、`SOUL.md`、`TOOLS.md`、`IDENTITY.md`、`USER.md`、`HEARTBEAT.md`、`MEMORY.md` 等 profile 文件表达行为规则、身份、风格、工具说明、用户画像、主动行为和记忆种子。当前这些文件若由 runtime 或 application 直接拼接，会导致上下文工程与具体 agent 目录结构强耦合，也难以审计注入优先级、预算和安全边界。

本提案把 profile 文件加载实现为 `ContextProvider`，让 agent profile 通过标准 candidate 进入 composer，而不是直接修改 prompt。

## What Changes

- 新增 `ProfileFileContextProvider`，按配置扫描 agent/workspace profile 文件。
- 支持固定 profile 文件类型：`AGENTS.md`、`SOUL.md`、`TOOLS.md`、`IDENTITY.md`、`USER.md`、`HEARTBEAT.md`、`MEMORY.md`。
- 定义 profile 文件优先级、cache class、默认注入策略和预算策略。
- 使用 Template Method 封装安全加载流程，使用 Strategy 替换优先级、扫描、截断和分类策略。
- `AGENTS.md`、`SOUL.md` 默认高优先级；`TOOLS.md`、`IDENTITY.md` 默认中优先级；`USER.md`、`HEARTBEAT.md` 默认低优先级或按阶段注入。
- `MEMORY.md` 默认作为记忆种子/审计入口，不自动等价为长期记忆完整注入。
- 所有 profile 注入必须进入 `ContextReport`。

## Impact

- Affected specs: `agent-profile-context`
- Affected code:
  - `macaca/crates/macaca-context`
  - agent/application manifest 或 profile loader 配置解析路径
  - `macaca-framework`、`macaca-runtime` 通过 context facade 间接受影响
- Dependencies:
  - 依赖 `add-context-composer-foundation`。
- Compatibility:
  - 不删除现有 persona/system prompt 配置。
  - 迁移后旧 profile prompt helper 标记 deprecated，不删除。
