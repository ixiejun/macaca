## Context

OpenClaw 的 skill 系统采用 AgentSkills-compatible `SKILL.md`：每个 skill 是一个目录，包含 `SKILL.md` frontmatter 和 markdown 指令。运行时只把 name、description、location 作为 catalog 注入 prompt，模型在任务匹配时再读取 skill 文件和相关资源。这个模型比把全部技能内容塞进 system prompt 更适合长期运行的多 agent 系统。

Macaca OS 当前已有 `macaca-skill` crate，包含 `AgentSkill`、`SkillCatalog`、`discovery` 和 progressive disclosure 注释，但还没有成为所有 traced agent 的统一运行时能力。此次变更要把它提升为 framework-level primitive。

## Goals

- 所有 application 的所有 agent 在 traced framework 入口都获得统一 skill catalog。
- Skill 选择和读取遵循标准 AgentSkills progressive disclosure：catalog -> instructions -> resources。
- Skill 可见性、启用状态、依赖条件和 prompt 注入结果可解释、可追踪、可复现。
- Session/resume/后台任务使用冻结的 skill snapshot，不因运行中配置变更产生非确定性。
- 与 OpenClaw/AgentSkills 的格式和生态兼容，避免 Macaca 自造封闭格式。
- 为未来 marketplace/install/update 留接口，但本次不实现市场功能。

## Non-Goals

- 不做 ClawHub 或其他 skill marketplace 的搜索、安装、更新。
- 不实现自动 skill 生成、审批、quarantine 工作流。
- 不废弃 YAML executable skills；本次只明确其与 knowledge skills 的边界。
- 不要求一次性把已有 application 的所有能力文档迁移成 `SKILL.md`。

## Architecture

### Skill Sources

Macaca SHALL load AgentSkills-compatible skills from the following sources, highest precedence first:

1. Session/app workspace skills: `<workspace>/skills`
2. Project/application agent skills: `<app_dir>/.agents/skills`
3. Application skills: `<app_dir>/skills`
4. User agent skills: `~/.agents/skills`
5. Macaca central skills: `~/.macaca/skills`
6. Bundled skills shipped by Macaca
7. Extra skill directories from config

If the same skill name appears in multiple sources, the higher-precedence source wins.

### Data Model

Introduce/extend these runtime concepts:

- `AgentSkill`: parsed standard skill metadata, canonical path, base dir, source, source scope.
- `SkillEntry`: `AgentSkill` plus parsed frontmatter, normalized metadata, invocation policy, exposure policy.
- `SkillPolicy`: per-application and per-agent visibility rules.
- `SkillSnapshot`: frozen prompt string, resolved skill metadata, source/version marker, and per-agent filter used for a session/run.
- `SkillRuntime`: loader/filter/snapshot service used by framework runner, loop manager, cron/background runs, and API status endpoints.

### Frontmatter

`SKILL.md` MUST support:

```markdown
---
name: browser-automation
description: Use browser tools for multi-step web research and extraction.
---
```

Macaca-specific metadata SHOULD use `metadata.macaca`. For ecosystem compatibility, the runtime SHALL also recognize equivalent `metadata.openclaw` fields when `metadata.macaca` is absent:

- `always`
- `emoji`
- `homepage`
- `primaryEnv`
- `os`
- `requires.bins`
- `requires.anyBins`
- `requires.env`
- `requires.config`
- `install`

Invocation policy frontmatter:

- `user-invocable`
- `disable-model-invocation`
- `command-dispatch`
- `command-tool`
- `command-arg-mode`

### Prompt Injection

`FrameworkRunner` SHALL call `SkillRuntime` during traced agent construction. The resulting catalog prompt SHALL be appended to the agent system prompt after persona/capabilities/workspace paths.

Prompt format SHOULD remain compatible with OpenClaw/AgentSkills:

```xml
<available_skills>
  <skill>
    <name>...</name>
    <description>...</description>
    <location>...</location>
  </skill>
</available_skills>
```

The prompt MUST instruct the agent:

- Use skills only when task matches name/description.
- Read `location` before applying a skill.
- Resolve relative paths against the skill base directory.
- Do not assume unlisted skills exist.

### Snapshot Semantics

At session/run creation, Macaca SHALL build a `SkillSnapshot` per agent. The snapshot SHALL be reused for:

- browser refresh / session reload
- coordinator resume
- worker retry
- plan loop review/evaluation
- cron/background runs

New sessions may pick up changed skill files/config. Existing sessions keep their snapshot unless explicitly refreshed by future API.

### Security

The runtime SHALL:

- Reject `SKILL.md` paths whose resolved realpath escapes the configured source root.
- Apply size limits to `SKILL.md` and source directories.
- Skip `node_modules`, `.git`, build artifacts, and hidden unsafe dirs during discovery.
- Sanitize env override keys and block dangerous host env mutation.
- Never inject secrets into prompts.
- Treat `install` metadata as descriptive only until an installer is explicitly implemented.

### Observability

Skill runtime events SHALL be traceable:

- `skill_catalog_built`: agent, session, visible count, filtered count.
- `skill_filtered`: skill name, reason such as disabled, missing_bin, missing_env, os_mismatch, denied_by_policy.
- `skill_snapshot_created`: agent, session, snapshot version/hash.
- `skill_used` or `skill_file_read`: emitted when an agent reads a skill `SKILL.md` location through framework tools where detectable.

These events SHOULD appear in the agent tab trace and persisted EventLog.

## Decisions

- Decision: Make `SKILL.md` knowledge skills first-class and keep YAML executable skills separate.
  - Rationale: Knowledge skills teach the model; executable skills are tools. Mixing them hides policy and safety boundaries.
- Decision: Use snapshot-by-session instead of always live reload.
  - Rationale: Autonomous long-running tasks need reproducibility. Live reload can make resumed agents see a different capability set mid-goal.
- Decision: Support `metadata.macaca` and OpenClaw-compatible `metadata.openclaw`.
  - Rationale: Macaca needs its own namespace but should consume existing ecosystem skills without rewriting them.
- Decision: Defer marketplace/install.
  - Rationale: Search/install/update introduces network trust, package safety, lockfiles, and UX work. Runtime support must land first.

## Risks / Trade-offs

- Prompt size may grow with many skills.
  - Mitigation: count/char budgets, compact catalog fallback, per-agent allowlists.
- Agents may overuse irrelevant skills.
  - Mitigation: explicit trigger guidance, descriptions, per-agent policy, trace visibility.
- Skill env overrides can leak secrets.
  - Mitigation: never prompt secrets, inject only runtime env, sanitize dangerous keys.
- Existing apps may rely on current YAML skill behavior.
  - Mitigation: keep executable skill registry unchanged and add tests for both paths.

## Migration Plan

1. Add `SkillRuntime` around existing `SkillCatalog`/discovery without changing existing app behavior.
2. Extend parser and metadata model while preserving current `name/description` support.
3. Add snapshot creation and prompt injection at traced agent construction.
4. Add status/debug API and trace events.
5. Add tests and migrate selected existing app skills only after runtime behavior is stable.

## Open Questions

- Whether `skills.load.extraDirs` belongs in app manifest, global config, or both.
- Whether skill snapshots should be persisted in EventLog, framework session store, or a dedicated skill snapshot store.
- Whether detecting `skill_used` should rely on file_read path matching first, or introduce an explicit `activate_skill` tool later.
