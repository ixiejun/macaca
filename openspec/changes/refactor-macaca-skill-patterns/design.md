## Context

`macaca-skill` supports two skill families:

- Knowledge skills from `SKILL.md`, exposed through prompt catalog and snapshots.
- Executable skills from YAML, exposed as `macaca_tools::Tool`.

This change keeps existing behavior but introduces stable primitives so upper crates can migrate without directly depending on concrete loader/filter/tool branches.

## Goals

- Preserve behavior 1:1 for existing `SkillRuntime`, `SkillRegistry`, `SkillCatalog`, `SkillProvisioner`, and `SkillTool` callers.
- Add Strategy/Chain primitives for metadata gating.
- Add Factory/Registry primitives for discovery sources.
- Add Memento primitives for executable registry snapshots.
- Add Adapter/Proxy primitives for executable tool exposure.
- Add State primitives for skill provisioning/runtime handle status.

## Non-Goals

- Do not migrate `macaca-web` or `macaca-runtime-host` consumers in this change.
- Do not implement marketplace install/search/update.
- Do not move MCP lifecycle ownership into `macaca-skill`; MCP remains Agent OS runtime responsibility.

## Decisions

- Legacy APIs remain available and are marked `#[deprecated]` only after new additive APIs exist.
- Policy evaluation returns stable reason strings already used by status APIs: `denied_by_policy`, `disabled_model_invocation`, `os_mismatch`, `missing_bin`, `missing_env`, `missing_config`.
- Runtime handles model lifecycle state but do not spawn long-lived MCP/browser resources.
