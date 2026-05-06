# Design: Skills 与 MCP Capability Context

## Context

Macaca 需要把 skills 和 MCP 变成 Agent OS 能力面，而不是某个 application 的定制 prompt。Skill runtime 已有 catalog/snapshot/progressive disclosure，MCP runtime 已有 server/tool/resource/prompt 发现和执行边界。上下文工程层应只消费这些系统暴露的快照/摘要，不拥有 skill discovery 或 MCP transport。

## Goals / Non-Goals

Goals:

- 将 skill/MCP/tool 能力统一建模为 capability candidates。
- 保持能力摘要紧凑、确定性、可预算。
- 保持 `SKILL.md` body 和 MCP resource content 按需加载。
- 避免 skill 直接绑定具体 MCP server。
- 支持能力命名空间、冲突消解和 report。

Non-Goals:

- 不实现 skill marketplace。
- 不实现 MCP server transport。
- 不在本提案中改变 tool execution policy。
- 不将完整 skills/MCP resources 全量注入 prompt。

## Decisions

### Decision 1: CapabilitySnapshot 是 Adapter 输入

skill runtime 和 MCP runtime 分别输出自己的 snapshot/registry。context 层通过 adapter 转换为 `CapabilityCandidate`，不反向调用 discovery 或 transport。

### Decision 2: Capability tree 使用 Composite

capability context 可表达：

- skill catalog entries
- runtime tools
- MCP tools
- MCP resources
- MCP prompts
- declared dependencies

Composite 结构便于命名空间、分组、去重和 UI/report 展示。

### Decision 3: 默认只注入 compact index

默认 prompt 只包含必要字段：

- id/name
- description
- location 或 capability reference
- usage discipline
- trust/source
- namespace

完整 `SKILL.md` body、resource body、prompt body 只能通过后续按需读取或显式动态 context 进入。

### Decision 4: Skill/MCP 通过 capability id 解耦

skill 可声明需要某类 capability，例如 browser、filesystem、database、fetch，而不是直接引用某个 MCP server 名称。映射由 capability registry 或 policy 处理。

## Risks / Trade-offs

- Risk: capability index 过大。Mitigation: compact summary、budget、分页/按需展开、report skipped。
- Risk: MCP resource prompt injection。Mitigation: resources/prompts 默认 dynamic/untrusted/fenced。
- Risk: skill 与 MCP 强耦合。Mitigation: dependency by capability id，禁止 skill provider 直接操作 MCP transport。
- Risk: tool 名称冲突。Mitigation: namespace、dedup、collision diagnostics。

## Migration Plan

1. 定义 capability context DTO 和 provider adapters。
2. 接入 skill snapshot 和 MCP registry snapshots。
3. 默认替代现有 direct skill/MCP prompt 注入。
4. 标记旧 helper deprecated。
5. 增加 report 和 tests。
