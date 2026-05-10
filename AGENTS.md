## 工作约定
- 在更改架构或行为之前，始终先理解当前代码。
- 优先使用小的、可审查的、可逆的变更。
- 除非必要，避免引入新的依赖。
- 如果行为发生变更，优先在编写代码之前更新规范。
- 优先遵循现有项目约定，而非发明新模式。
- 所有代码都必须有详尽的注释，解释代码功能，运行原理。

# Rust / Macaca 默认规范
- 禁止巨型文件，每个文件代码行数最多不得超过500行，如果代码过多就要考虑是否没有采用设计模式，没有对代码功能进行科学的拆分，在编程前缺少对架构进行优雅的设计
- 用Superpowers执行 brainstorm、write-plan，用OpenSpec编写proposal、design、tasks、spec之前必须先考虑能否采用/Users/quantum/Code/dev/agent/macaca/docs/design_patterns.md里的某种设计模式
- 对于可用多种设计模式实现的，优先考虑可扩展性和性能开销
- 不可过度设计
- Macaca是一个7*24小时全自动执行task，人工零干预的agent os,上层需要运行多元，功能完全不一样的application，不可编写专门化，特定化的代码，代码需要对所有的application具有通用性
- Macaca是一个基础设施，是一个操作系统，你可以把它理解为agent领域的linux
- 禁止在代码中硬编码workflow，app name，driver name和任何应用相关，业务相关的名称
- 优雅永不过时，简洁永不过时

# 强制工作流
对于任何业务功能、行为变更、接口变更或非平凡重构：
- 1. 阅读当前代码和相关模块。
- 2. 根据任务需求用Superpowers执行 brainstorm，给出可选方案与风险
- 3. 从可选方案里选择最合理的执行方案撰写实施计划： write-plan
- 4. 根据Superpowers产出的plan首先创建或更新 OpenSpec 产物。
- 5. 审查提案/设计/任务的一致性。
- 6. 以小步骤实施。
- 7. 通过测试和契约检查进行验证。
- 8. 仅在代码和规范对齐后归档。
# OpenSpec 规则
- `openspec/specs/` 是基线事实来源。
- `openspec/changes/` 存储提议的增量变更。
- 使用 `proposal.md` 说明原因。
- 使用 `design.md` 说明技术决策和权衡。
- 使用 `tasks.md` 说明执行步骤。
- 行为变更使用增量规范。

# Superpowers 规则
对于模糊、广泛、有风险或跨领域的工作：
- 在设计之前进行头脑风暴
- 在实施之前编写计划
- 逐步执行计划 

推荐顺序：

- /superpowers:brainstorm
- /superpowers:write-plan
- /superpowers:execute-plan
<!-- OPENSPEC:START -->

# OpenSpec Instructions

These instructions are for AI assistants working in this project.

Always open `@/openspec/AGENTS.md` when the request:
- Mentions planning or proposals (words like proposal, spec, change, plan)
- Introduces new capabilities, breaking changes, architecture shifts, or big performance/security work
- Sounds ambiguous and you need the authoritative spec before coding

Use `@/openspec/AGENTS.md` to learn:
- How to create and apply change proposals
- Spec format and conventions
- Project structure and guidelines

Keep this managed block so 'openspec update' can refresh the instructions.

<!-- OPENSPEC:END -->

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **agent** (403393 symbols, 1049932 relationships, 300 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/agent/context` | Codebase overview, check index freshness |
| `gitnexus://repo/agent/clusters` | All functional areas |
| `gitnexus://repo/agent/processes` | All execution flows |
| `gitnexus://repo/agent/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
