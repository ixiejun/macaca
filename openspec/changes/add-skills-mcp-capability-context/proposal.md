# Change: 增加 Skills 与 MCP Capability Context

## Why

Macaca 完整兼容 Agent Skills 生态并支持 MCP 协议，但这些能力只有通过上下文工程以纪律化方式进入模型，agent 才能正确知道“有哪些能力、何时使用、如何按需展开”。如果直接把完整 `SKILL.md`、MCP resource 或 tool schema 全量塞进 prompt，会造成上下文膨胀、prompt injection 风险和技能/MCP 强耦合。

本提案将 skills、MCP tools/resources/prompts 和 runtime tools 暴露为紧凑、可审计、可预算的 capability context。

## What Changes

- 新增 `SkillContextProvider`、`McpContextProvider` 或等价 capability adapters。
- 默认注入紧凑 capability index，不默认注入完整 `SKILL.md` body 或 MCP resource content。
- skill catalog 保持 progressive disclosure：先暴露 name/description/location/usage discipline，匹配任务后按需读取。
- MCP capability context 暴露 server/tool/resource/prompt 摘要、namespace、trust、usage constraints 和安全边界。
- skills 可声明依赖 capability id，而不是直接依赖具体 MCP server internals。
- 对 capability name collision 做 namespace/dedup。
- MCP resources/prompts 默认 untrusted、dynamic、fenced。
- 所有 capability 注入和跳过原因进入 `ContextReport`。

## Impact

- Affected specs: `capability-context`
- Affected code:
  - `macaca/crates/macaca-context`
  - `macaca/crates/macaca-skill`
  - MCP runtime/registry 相关 crate 或模块
  - `macaca-framework`、`macaca-runtime` 通过 context facade 间接受影响
- Dependencies:
  - 依赖 `add-context-composer-foundation`。
  - 复用既有 `add-agent-skills-runtime`、`add-agent-os-mcp-runtime`、`add-skill-backed-mcp-runtime`。
- Compatibility:
  - 不删除现有 tool schema 暴露路径。
  - 旧 direct skill/MCP prompt helper 迁移后标记 deprecated，不删除。
