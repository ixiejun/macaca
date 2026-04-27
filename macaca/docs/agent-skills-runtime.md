# AgentSkills Runtime

Macaca OS uses AgentSkills-compatible `SKILL.md` directories as the standard knowledge skill format for all applications and agents.

## Layout

Each knowledge skill is a directory containing `SKILL.md`:

```markdown
---
name: browser-automation
description: Use browser tools for multi-step web research and extraction.
metadata:
  macaca:
    requires:
      bins: [node]
      env: [SEARCH_API_KEY]
---

# Browser Automation

Instructions for when and how the agent should use this skill.
```

`metadata.openclaw` is also accepted when `metadata.macaca` is absent, so existing OpenClaw/AgentSkills ecosystem skills can be reused without rewriting.

## Source Precedence

When duplicate skill names exist, higher precedence wins:

1. `<workspace>/skills`
2. `<app_dir>/.agents/skills`
3. `<app_dir>/skills`
4. `~/.agents/skills`
5. `~/.macaca/skills`
6. bundled skills
7. extra configured directories

## Knowledge Skills vs Executable Skills

Knowledge skills (`SKILL.md`) teach the model how to do work. They are injected into the prompt as a catalog and then read on demand.

Executable skills (`*.yaml` / `*.yml`) remain tool definitions and are loaded through `SkillRegistry`. A `SKILL.md` file never registers an executable tool by itself.

## Skill 与 MCP 的职责边界

`SKILL.md` 的职责是知识和使用说明：告诉 agent 何时使用某类能力、如何判断结果、如何组织工作流。它不拥有 MCP 进程生命周期，也不直接启动浏览器、搜索、数据库等外部服务。

MCP server 的职责是可执行工具能力：由 Agent OS 级 `McpRuntimeManager` 统一发现、注册、启动、隔离、关闭和观测。Skill metadata 中的 `metadata.macaca.mcpServers` / `metadata.openclaw.mcpServers` 只作为 discovery hint，被导入为 OS MCP definition；真正进入 agent toolkit 时仍统一经过 MCP registry/runtime。

因此最终边界是：

- Skill runtime：负责 skill 发现、metadata gating、prompt catalog、snapshot 持久化。
- MCP registry/runtime：负责 MCP server definition、transport、policy、lifecycle、resource cleanup、status API。
- Framework toolkit：负责把 MCP tools 作为普通 tool 注入，并继续走 trace middleware。

## Runtime Behavior

Every traced framework agent receives an `<available_skills>` catalog in its system prompt. The catalog contains skill name, description, and location only. The agent must read the skill `SKILL.md` before applying the skill instructions, and relative resources are resolved against the skill directory.

Skill snapshots are frozen per session and agent in the framework session store. Resume/retry/review flows reuse the stored snapshot; new sessions can see updated skills.

## Policy and Gating

Agents can define optional skill policy:

```yaml
skills:
  allow: [browser-automation, markdown-writing]
  deny: [unsafe-local-admin]
```

Supported metadata gates:

- `always`
- `os`
- `requires.bins`
- `requires.anyBins`
- `requires.env`
- `requires.config`
- `primaryEnv`
- `homepage`
- `emoji`

Secrets and environment values are never rendered into prompts.

## Observability

The runtime records these EventLog events when session context is available:

- `skill_catalog_built`
- `skill_snapshot_created`
- `skill_file_read`
- `mcp_server_resolved`
- `mcp_server_starting`
- `mcp_server_ready`
- `mcp_tools_registered`
- `mcp_server_failed`
- `mcp_server_closed`

For backward compatibility, skill-backed MCP readiness can still be surfaced through `skill_mcp_*` aliases, but the underlying runtime path is the Agent OS MCP runtime, not a skill-local subprocess launcher.

The skills status API is available at:

```text
GET /api/apps/{app_id}/skills
GET /api/apps/{app_id}/skills?agent=planner
```

The response includes visible skills, filtered skills, and filter reasons.

## Deferred

Marketplace install/search/update is intentionally not part of the first runtime implementation. The runtime accepts standard skill folders now; a future ClawHub-compatible provider can install into the same workspace `skills/` directory.
