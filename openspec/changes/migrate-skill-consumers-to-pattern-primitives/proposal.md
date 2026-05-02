# Change: Migrate macaca-skill consumers to pattern primitives

## Why
`macaca-skill` now exposes design-pattern primitives for policy, source discovery, registry snapshots, tool adapters, and lifecycle handles. Upper crates still call deprecated direct APIs and duplicate snapshot/source construction logic, which prevents clean migration and keeps skill behavior scattered across web/app/test layers.

## What Changes
- Add thin consumer-facing skill request/facade APIs in `macaca-skill`.
- Migrate `macaca-web` skill startup, snapshot construction, skill status, and skill-backed MCP paths to those APIs.
- Migrate `macaca-app::SkillLoader` source inventory to the canonical skill source primitives where behavior can remain 1:1.
- Migrate integration tests away from deprecated `SkillRegistry` and `SkillTool` constructors.
- Keep deprecated APIs in `macaca-skill` for compatibility, but remove upper-crate usages.

## Impact
- Affected specs: macaca-skill-consumers
- Affected code: `crates/macaca-skill`, `crates/macaca-web`, `crates/macaca-app`, `crates/macaca-integration-tests`
- Runtime behavior must remain 1:1 for visible skills, filtered skills, snapshot persistence, skill-backed MCP, and executable skill tool execution.
