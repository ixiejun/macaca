## Context

AgentSkills and MCP solve different layers:

- AgentSkills `SKILL.md`: tells the model when and how to use a capability.
- MCP server: exposes actual executable tools such as `browser_navigate`.

OpenClaw handles this by shipping plugin/tool runtimes alongside skills. SkillHub can distribute a standalone `SKILL.md` that describes an MCP package, but Macaca still needs a runtime bridge that starts/connects the MCP server and merges its tools into the agent toolkit.

The `playwright-mcp` validation proves the current gap: knowledge skill injection works, tool availability does not.

## Goals

- Skill-backed MCP tools SHALL be available to all traced framework agents when the corresponding skill is visible and eligible.
- The same skill policy and metadata gating model SHALL control both the skill prompt and its MCP tools.
- Skill-backed MCP server startup and tool exposure SHALL be observable and persisted.
- The implementation SHALL be generic and not hardcode application names such as FULLSTACK-AUTODEV or NEWSROOM-AUTOWRITER.
- `playwright-mcp` SHALL work as the first compatibility target.

## Non-Goals

- No marketplace install/update.
- No automatic package installation.
- No app-specific special cases.
- No requirement that every skill has tools; instruction-only skills remain valid.

## Metadata Model

Macaca-owned skill metadata:

```yaml
metadata:
  macaca:
    requires:
      bins: ["playwright-mcp"]
    mcpServers:
      playwright:
        command: "playwright-mcp"
        args: ["--headless"]
        transport: "stdio"
        toolPrefix: "browser_"
```

For ecosystem compatibility, when `metadata.macaca.mcpServers` is absent, Macaca MAY resolve server definitions from:

- application/global MCP config keyed by skill name
- a local compatibility registry keyed by skill package metadata, e.g. `@playwright/mcp`

The compatibility registry is not app-specific. It maps known package/tool ecosystems to MCP server launch definitions and can be extended without changing app logic.

## Runtime Flow

1. `SkillRuntime` builds per-agent snapshot.
2. `SkillMcpRuntime` inspects visible snapshot skills.
3. For each skill with MCP config or registry mapping:
   - verify binary/package dependency exists
   - start/connect server
   - discover tools
   - apply per-agent tool namespace policy
   - register tools into framework toolkit
4. Framework runner builds agent with normal tools + eligible skill-backed MCP tools.
5. Tool calls and server lifecycle events are traced and persisted.

## Policy

- Application-level config can disable all skill-backed MCP tools.
- Agent-level config can allow/deny skill names and tool namespaces.
- If a skill is denied or filtered, its MCP tools MUST NOT be registered.
- If an MCP server fails, the skill may still appear as knowledge if policy allows, but status MUST report tool runtime failure.

## Observability

Events:

- `skill_mcp_resolved`
- `skill_mcp_starting`
- `skill_mcp_ready`
- `skill_mcp_failed`
- `skill_mcp_tools_registered`
- normal framework `tool_call` / `tool_result` for MCP tool invocation

Status API should show:

- skill name
- server id
- command/args with secrets redacted
- state
- exposed tool names
- failure reason

## Risks / Trade-offs

- MCP subprocesses can hang or leak resources.
  - Mitigation: lifecycle manager, timeout, cancellation, per-session cleanup.
- Tool namespace collisions can confuse agents.
  - Mitigation: deterministic prefixing/collision rejection.
- Starting MCP servers for every visible skill can be expensive.
  - Mitigation: lazy start on first relevant run, cache per app/session with health state.
- Standalone SkillHub packages may not include machine-readable MCP config.
  - Mitigation: support `metadata.macaca.mcpServers` and a generic compatibility registry.

## Rollout Plan

1. Add metadata structs and config parsing for `metadata.macaca.mcpServers`.
2. Add compatibility registry entry for `@playwright/mcp` / `playwright-mcp`.
3. Implement MCP client wrapper as framework tools.
4. Wire skill-backed MCP tools into traced framework toolkit.
5. Add status and trace events.
6. Validate with `/Users/quantum/.macaca/skills/playwright-mcp`.
