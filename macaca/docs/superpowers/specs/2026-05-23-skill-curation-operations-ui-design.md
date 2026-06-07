# Skill Curation Operations UI Design

## Context

The self-evolving skills research recommends a final shell slice that exposes
Skill governance, curation dry-run, alias, and draft proposal state without
moving curation semantics into Web or the frontend. The latest implementation
already provides the required Skill service and SDK read commands.

## Selected Approach

Use the existing Skill service facade as the semantic boundary. Add a thin Web
route that aggregates `governance_snapshot`, `curation_dry_run`,
`alias_snapshot`, and `skill_experience_snapshot` into one sanitized operations
view. Add a frontend panel inside the existing application operations dialog.

## Alternatives Considered

- Put curation logic in the frontend. Rejected because it would make the shell
  classify stale, pinned, superseded, or draft records.
- Add new Skill service commands before UI. Rejected for this slice because the
  current SDK already has the needed read-only commands.
- Build a standalone page. Rejected because application operations already
  hosts application-scoped OS controls.

## Architecture

- Web route is an Adapter over `SystemSkillClient`.
- Frontend API/types are presentation DTOs only.
- UI panel is read-only except for refreshing and requesting a service-owned
  dry-run. It never writes `SKILL.md`, aliases, lifecycle state, or proposals.

## Trace, Audit, And Logs

Each route call creates a trace id and logs the bounded counts returned by the
Skill service. Returned payloads contain metadata, counters, recommendations,
aliases, and proposals only; no raw skill instructions, prompts, manifests,
provider payloads, package bytes, or secrets are exposed.

## Verification

Run OpenSpec validation, Rust checks for `macaca-web`, focused Skill provider
tests, frontend lint/build, and GitNexus change detection before completion.
