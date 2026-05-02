# Change: Refactor macaca-skill with design-pattern primitives

## Why
`macaca-skill` is the Agent OS foundation for standard skills, metadata gating, snapshot recovery, executable skill exposure, and skill lifecycle. Current implementations mix discovery, filtering, prompt formatting, tool wrapping, and provisioning in a few concrete modules, which makes future MCP/skill/runtime expansion fragile.

## What Changes
- Add additive-first skill exposure policy chain primitives.
- Add source factory primitives for skill discovery sources.
- Add registry snapshot/reload primitives for executable skills.
- Add skill tool adapter/proxy primitives for executable skill tool exposure.
- Add runtime handle/state primitives for provisioning lifecycle.
- Mark legacy direct APIs as deprecated but keep them callable for migration.

## Impact
- Affected specs: macaca-skill-core
- Affected code: `crates/macaca-skill/src/*`
- No application-specific logic, workflow-specific logic, or driver-name hardcoding is introduced.
