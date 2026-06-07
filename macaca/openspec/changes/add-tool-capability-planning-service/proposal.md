# Change: Add tool capability planning service

## Why

Applications and agents need deterministic tool plans with visible tools, hidden diagnostics, family/toolset policy, availability reasons, conflicts, and compact context integration. Current toolkit assembly spreads this logic across Web shell, framework toolkit construction, provider services, and compatibility paths.

Planning must be service-owned and application-neutral so every Macaca application can request generic capabilities without OS-layer business branches.

## What Changes

- Add `tool.catalog.plan`, `tool.catalog.snapshot`, and `tool.toolset.resolve` behavior.
- Add descriptor contributors for existing Driver, Skill, MCP, Memory, Task, Scheduler, Gateway, workspace, and runtime tools.
- Add data-driven tool family and toolset resolution.
- Add availability expression evaluation and stable hidden diagnostics.
- Add conflict handling and provider status summaries.
- Add compact Context provider for tool capability indexes.
- Add generic application manifest support for tool families and toolsets while preserving exact `allowed_tools` compatibility.

## Impact

- Affected specs: `tool-capability-planning`, `context-composer`
- Affected code: `macaca-runtime-host`, `macaca-context`, `macaca-app`, `macaca-web`
- Depends on: `add-tool-capability-contracts`

## Constraints

- `service.tool` coordinates planning but does not own concrete provider lifecycle.
- Web/CLI/frontend must remain shell adapters.
- Context must receive compact indexes only, not raw tool docs, raw MCP resources, raw provider payloads, or unbounded schemas.
- Toolsets and families must be data-driven; no application-name branching is allowed.
