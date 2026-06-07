## Context

The previous `macaca-skill` refactor introduced primitive building blocks but intentionally did not migrate upper consumers. This change migrates consumers without changing behavior.

## Goals

- Remove deprecated `SkillRegistry::load_from_directory`, `SkillRegistry::instantiate_tool`, `SkillRegistry::instantiate_all_tools`, and `SkillTool::new` usage from upper crates.
- Centralize snapshot input construction behind `SkillSnapshotRequest`.
- Keep skill policy resolution in upper crates because `macaca-skill` must not depend on app manifest types.
- Keep MCP lifecycle in Agent OS MCP runtime.

## Non-Goals

- Do not remove deprecated APIs.
- Do not change skill metadata schema.
- Do not implement marketplace install/update.
- Do not change application-specific manifests.

## Decisions

- `SkillRuntimeFacade` accepts only generic paths, policy, limits, and agent identity.
- `macaca-web` remains responsible for resolving app/agent skill policy from application manifest.
- `ExecutableSkillToolSet` produces the same `Box<dyn macaca_tools::Tool>` values as the old startup path, but internally uses `SkillToolAdapter`.
- Deprecated APIs remain callable only as compatibility wrappers; upper crates should not call them.
