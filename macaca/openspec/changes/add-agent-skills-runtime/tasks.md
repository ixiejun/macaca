## 1. Discovery and Data Model

- [x] 1.1 Extend `AgentSkill` with source, source scope, canonical realpath, base dir, and optional homepage/emoji metadata.
- [x] 1.2 Add `SkillEntry`, `SkillPolicy`, `SkillInvocationPolicy`, `SkillExposure`, and `SkillSnapshot` types.
- [x] 1.3 Implement standard source precedence across workspace/app/user/central/bundled/extra dirs.
- [x] 1.4 Add path escape protection, max candidate limits, max file size limits, and skip-dir filtering.
- [x] 1.5 Preserve current YAML executable skill loading as a separate path.

## 2. Frontmatter and Gating

- [x] 2.1 Extend `SKILL.md` parser for optional frontmatter fields and single-line metadata JSON/YAML maps.
- [x] 2.2 Support `metadata.macaca` and compatible fallback to `metadata.openclaw`.
- [x] 2.3 Implement OS, required bins, any bins, required env, required config, and `always` gating.
- [x] 2.4 Implement app-level and agent-level skill allowlist/denylist/disable behavior.
- [x] 2.5 Ensure secrets/env values are never rendered into prompt text.

## 3. Runtime Snapshot and Prompt Injection

- [x] 3.1 Create `SkillRuntime` service responsible for loading, filtering, prompt formatting, and snapshot creation.
- [x] 3.2 Add per-agent `SkillSnapshot` creation for every traced agent run.
- [x] 3.3 Persist/reuse snapshots for session reload, coordinator resume, worker retry, planner review, and background runs.
- [x] 3.4 Inject `<available_skills>` into every traced framework agent system prompt.
- [x] 3.5 Add compact catalog fallback and prompt budget limits.

## 4. Resource Access and Tool Integration

- [x] 4.1 Update system prompt guidance so agents read matching `SKILL.md` before applying a skill.
- [x] 4.2 Ensure relative skill resources are resolved against skill base dir.
- [x] 4.3 Add safe helper for checking whether a file read belongs to a visible skill.
- [x] 4.4 Emit skill usage trace when the agent reads a visible skill file.

## 5. API and Observability

- [x] 5.1 Add skill status endpoint showing visible, filtered, and disabled skills per app/agent.
- [x] 5.2 Add filter reasons: disabled, denied_by_policy, missing_bin, missing_env, missing_config, os_mismatch, path_escape, oversized.
- [x] 5.3 Emit EventLog/trace events for catalog build, snapshot creation, filtered skills, and skill usage.
- [ ] 5.4 Update frontend trace rendering if new skill events need labels/cards.

## 6. Tests

- [x] 6.1 Unit test frontmatter parsing and OpenClaw-compatible metadata fallback.
- [x] 6.2 Unit test source precedence and same-name override behavior.
- [x] 6.3 Unit test gating for OS, bins, env, config, allowlist, denylist, and disable-model-invocation.
- [x] 6.4 Unit test snapshot stability across config/skill changes.
- [ ] 6.5 Integration test that coordinator/planner/worker agents all receive skill catalog through traced entry.
- [x] 6.6 Integration test that YAML executable skills still load as tools.
- [ ] 6.7 E2E test that a workspace skill is visible, read by the agent, traced, persisted, and reloadable after browser refresh.

## 7. Documentation

- [x] 7.1 Document Macaca standard skill layout and supported metadata.
- [x] 7.2 Document skill source precedence and per-agent policy.
- [x] 7.3 Document difference between AgentSkills knowledge skills and YAML executable skills.
- [x] 7.4 Document security model and why marketplace/install is deferred.
