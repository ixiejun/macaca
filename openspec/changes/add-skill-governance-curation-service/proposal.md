# Change: Add Skill Governance And Curation Service Slice

## Why

Macaca agents must be able to improve their reusable procedural knowledge without allowing skill directories to grow into unbounded, untraceable prompt artifacts. The existing Skill service already owns AgentSkills-compatible discovery and snapshots, so the next implementation slice should add OS-governed metadata, usage telemetry, and curation dry-run contracts behind the same service boundary.

## What Changes

- Add provider-neutral Skill governance lifecycle, provenance, usage telemetry, and curation report contracts.
- Extend the Skill service command surface with traced governance snapshot, usage recording, and deterministic curation dry-run commands.
- Keep all behavior generic: no application-specific skill names, workflows, providers, or business rules.
- Keep the first slice non-destructive: no automatic patch, merge, archive, or delete operation is executed by this change.

## Impact

- Affected specs: `skill-governance-curation`
- Affected code:
  - `macaca/crates/services/macaca-skill/src/*`
  - `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider.rs`
  - `macaca/crates/facade/macaca-sdk/src/skill_client.rs`
- Architecture constraints:
  - Kernel remains provider-neutral and does not own skill generation or curation.
  - Web/CLI remain shell adapters and do not own curation semantics.
  - Runtime-host remains the composition/adaptation boundary for the built-in provider.
