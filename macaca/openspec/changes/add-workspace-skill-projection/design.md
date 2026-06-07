## Context

Macaca is an Agent OS. Skills may be installed in several user-client directories, but Macaca-owned skills are the primary platform source and should win discovery conflicts. At runtime, an application agent should receive a stable workspace-local path that it can read directly with ordinary file tools.

## Goals

- Prefer Macaca-owned skills at `~/.macaca/skills`.
- Fall back to generic skills at `~/.agent/skills`.
- Fall back to common client directories: `~/.claude/skills`, `~/.codex/skills`, `~/.hermes/skills`, and `~/.openclaw/skills`.
- Create deterministic workspace-local skill projections under `available_skills`.
- Preserve canonical source locations for audit, collision handling, and path membership checks.

## Non-Goals

- Do not change skill allow/deny semantics.
- Do not introduce crypto-specific, agent-specific, or application-specific branches.
- Do not replace file-based skill reading with a new tool in this change.

## Design

The design uses a Builder-style runtime snapshot pipeline plus a projection adapter:

1. `SkillSourceSet::from_options` builds ordered source roots. Application/workspace roots remain available for app-local skills, while user-global roots follow the Macaca-first fallback order.
2. `SkillRuntime::build_snapshot` discovers and filters skills first, so only policy-visible skills are projected.
3. A projection step creates `workspace/available_skills/<stable-slug>/` and copies the selected skill directory there. The snapshot entry's model-facing `location` and `base_dir` point at the projection.
4. New `source_location` and `source_base_dir` fields retain the original canonical skill path. `path_belongs_to_snapshot_skill` accepts both projected and source skill directories so existing file policy remains compatible.

Copying the full skill directory is intentionally simple and traceable. It makes `SKILL.md` plus relative files such as scripts available without teaching the model a second resolution rule.

## Risks

- Projection adds file I/O to snapshot building. The implementation only projects visible prompt skills and removes stale projected directories before copying each skill.
- Symlinks could escape the source tree if copied naively. The implementation copies regular files and directories and skips symlinks.
- Existing callers that read `SkillSnapshotEntry.location` will now see the projected location when a workspace exists. The original source location remains available in the snapshot for diagnostics.
