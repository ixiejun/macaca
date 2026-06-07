# Change: Add Skill Curation Lifecycle Commands

## Why

Macaca now exposes Skill governance telemetry, curation dry-run reports, aliases,
draft experience proposals, and a read-only operations shell.  Operators and
future autonomy services still need a governed service command surface for the
first safe lifecycle mutations before approval-gated UI or automatic curation
can be added.

## What Changes

- Add traced Skill service commands for pin, unpin, archive, and restore.
- Keep the commands metadata-only: governance records change, but `SKILL.md`
  files, aliases, package content, and executable scripts are not modified.
- Deny archive of pinned skills until a future approval contract explicitly
  defines forced mutation semantics.
- Extend the SDK Skill facade and built-in runtime-host provider with structured
  logs and unavailable behavior.

## Impact

- Affected specs: `skill-governance-curation`
- Affected code:
  - `macaca/crates/services/macaca-skill/src/governance.rs`
  - `macaca/crates/services/macaca-skill/src/service_contract.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_state.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider.rs`
  - `macaca/crates/facade/macaca-sdk/src/skill_client.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_tests.rs`
