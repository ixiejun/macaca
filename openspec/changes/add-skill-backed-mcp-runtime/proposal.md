# Change: Add skill-backed MCP runtime support

## Why

Installing a standard AgentSkills `SKILL.md` is not enough when the skill describes tools that must be provided by an MCP server. The `playwright-mcp` SkillHub package was discovered, passed metadata gating, and was read by the model, but the agent could not call `browser_navigate` or `browser_snapshot` because Macaca did not connect the Playwright MCP server as runtime tools.

Macaca OS must support this pattern to be compatible with mature AgentSkills ecosystems: skills provide instructions, while tool providers such as MCP servers provide executable capabilities. A 7x24 autonomous agent OS needs both to be discoverable, traceable, policy-controlled, and available without manual per-session intervention.

## What Changes

- Add a skill-backed MCP runtime that can start/connect MCP servers declared by skill metadata or global/app MCP config.
- Extend skill metadata support with a Macaca-owned `metadata.macaca.mcpServers` block.
- Support compatible fallback for known skill packages that are instruction-only but declare install metadata for MCP packages, starting with a generic package-to-server mapping mechanism rather than app-specific hardcoding.
- Expose MCP tools from eligible skill-backed servers into the same framework toolkit used by coordinator/planner/worker agents.
- Add policy controls so applications/agents can allow or deny MCP-backed skills and their tool namespaces.
- Add runtime health/status diagnostics for each skill-backed server: not_configured, dependency_missing, starting, ready, failed.
- Emit trace/EventLog events when a skill-backed MCP server is resolved, started, connected, exposes tools, or fails.
- Keep market/search/install out of scope; this change consumes already-installed skills and already-installed binaries/packages.

## Impact

- Affected specs: `skill-backed-mcp-runtime`
- Related pending specs: `agent-skills-runtime`
- Affected code:
  - `macaca/crates/macaca-skill/src/*`
  - `macaca/crates/macaca-web/src/framework_toolkit.rs`
  - `macaca/crates/macaca-web/src/framework_runner.rs`
  - `macaca/crates/macaca-web/src/state.rs`
  - MCP/tool integration modules under `macaca-framework` or `macaca-tools`
  - frontend trace UI if new MCP/skill tool events need rendering

## Evidence From Current Validation

- Installed skill: `/Users/quantum/.macaca/skills/playwright-mcp/SKILL.md`
- Dependency installed: `@playwright/mcp`, binary available as `/opt/homebrew/bin/playwright-mcp`
- Skill status after OS compatibility fix: `playwright-mcp` visible from `macaca_central`
- Actual task result:
  - model read `/Users/quantum/.macaca/skills/playwright-mcp/SKILL.md`
  - model correctly reported missing runtime tools: `browser_navigate`, `browser_snapshot`, `browser_click`, etc.

## Out of Scope

- ClawHub/SkillHub marketplace search/install/update.
- Automatic installation of npm/brew/go/uv dependencies.
- Browser UI design changes beyond trace/status rendering.
