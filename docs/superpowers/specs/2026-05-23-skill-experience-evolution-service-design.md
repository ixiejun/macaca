# Skill Experience Evolution Service Design

## Context

`docs/macaca-agent-self-evolving-skills-research.md` recommends landing Macaca self-evolving skill support in small governed slices. The latest completed slice added Skill alias resolution, so the next safe step is draft-only experience evolution: convert verified task evidence into a typed proposal that can later become a skill draft or patch, without mutating active skill files.

## Options Considered

1. Add a separate Skill Evolution service now.
   - Benefit: clean future ownership for advanced providers.
   - Risk: adds a new service surface before storage, policy, approval, and curation apply semantics exist.
2. Extend `service.skill` with draft-only evolution commands.
   - Benefit: reuses the existing Skill service descriptor, SDK facade, trace-required provider, governance state, and tests.
   - Risk: the Skill provider can grow too broad unless state and command handling stay split by responsibility.
3. Generate draft files directly from task completion hooks.
   - Benefit: visible artifacts immediately.
   - Risk: violates the governance documents because filesystem mutation would bypass policy, memento, approval, and audit.

## Decision

Use option 2. Add provider-neutral evolution DTOs and a traced `skill.evolution.propose_from_task` command to `service.skill`. The built-in runtime-host provider stores proposals in the existing governance state helper and returns a sanitized draft result. The command is deterministic and non-destructive: it records candidate metadata, evidence references, classification, and recommended action, but does not create, patch, archive, or activate skills.

## Architecture

- Command pattern: all evolution input and output is typed in `macaca-skill`.
- Facade pattern: SDK users call `SystemSkillClient::propose_skill_experience`.
- State and Memento vocabulary: proposals are stored as draft governance records with captured timestamps and evidence ids.
- Specification pattern: command validation rejects candidates without reusable procedure summaries, evidence references, or trace context.
- Observer pattern: provider logs acceptance and completion with trace id, proposal id, recommended action, and accepted/rejected state.

## Boundaries

- Kernel remains unaware of experience extraction semantics.
- Runtime-host owns only the built-in provider adapter and replaceable in-memory strategy.
- SDK exposes provider-neutral calls and Null Object unavailable behavior.
- Web/CLI/applications can later display or approve proposals through the facade, but cannot own classification or mutation rules.
- No application-specific workflow, app name, provider name, model name, or business taxonomy is hardcoded.

## Acceptance

- OpenSpec validates the new change.
- Provider tests prove proposal creation is service-owned, sanitized, trace-backed, and non-mutating.
- SDK checks prove the facade compiles with unavailable behavior.
- Existing governance, alias, and descriptor tests continue to pass.
