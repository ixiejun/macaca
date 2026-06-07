# Change: Add Skill Experience Evolution Service Slice

## Why

Macaca agents need a governed path for turning verified task experience into reusable skill drafts, but direct skill file mutation would bypass policy, trace, audit, approval, and rollback requirements.

## What Changes

- Add provider-neutral Skill experience proposal DTOs under the existing Skill service contract.
- Add a traced `skill.evolution.propose_from_task` command that accepts sanitized verified task evidence and returns a draft proposal record.
- Store proposals in the built-in provider's governance state without creating, patching, archiving, or activating any skill files.
- Expose the command through the SDK Skill facade with structured unavailable behavior.

## Impact

- Affected specs: `skill-governance-curation`
- Affected code: `macaca-skill`, `macaca-runtime-host` Skill provider, `macaca-sdk` Skill client
- Boundary stance: service-owned and facade-consumed; no kernel, shell, or application-specific semantics.
