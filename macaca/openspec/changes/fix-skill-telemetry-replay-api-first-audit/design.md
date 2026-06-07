## Context

The built-in Skill provider already models governance changes as sanitized `SkillGovernanceEventRecord` values and can replay them into `SkillGovernanceReadModel`. The missing piece is a durable local memento. Restart package recovery only reconstructs the materialized package identity and proposal evidence; it intentionally cannot reconstruct usage counters from package files.

The Web shell already exposes `/skills/operations` and `/skills?agent=...`, but self-evolution audit tasks have no single canonical evidence surface. Agents can therefore produce stale filesystem-first reports that miss service-owned state.

## Goals

- Persist and replay usage telemetry events without moving governance semantics into Web, CLI, or application code.
- Keep the journal sanitized and bounded.
- Provide API-first audit/trigger verification that explicitly reports missing evidence.

## Non-Goals

- Do not implement a production Store/EventLog backend in this slice.
- Do not infer optimization quality from filesystem artifacts.
- Do not hardcode any application, workflow, provider, driver, or business-domain name.

## Technical Decisions

### Durable telemetry replay

Use the existing `SkillGovernanceStoreStrategy` as the Strategy boundary and add a local file-backed memento to `SkillProviderGovernanceState`. The provider appends each already-sanitized event as one JSON line. Startup replays valid lines into `SkillGovernanceReadModel`, restores the in-memory maps, and logs skipped invalid lines without failing the whole service.

The Web composition root passes a workspace-level journal path to the provider. This is composition wiring only; Web does not parse or interpret Skill governance events.

### API-first audit

Add a Web diagnostic adapter that gathers:

- Skill operations governance records from the Skill client.
- Registry/load-path visibility from the existing app skills route logic.
- Session observer evidence from bounded EventLog/session store metadata.

The adapter returns explicit pass/fail states with missing-evidence reasons. It remains a shell adapter: it calls canonical APIs and does not own curation, materialization, registry, or telemetry semantics.

## Risks

- A malformed JSONL line could block startup if replay is fail-closed. Mitigation: log and skip malformed lines while preserving valid earlier/later events.
- A very large journal could slow local startup. Mitigation: impose a first-slice replay cap and log truncation.
- API-first audit could become semantic ownership if it starts classifying lifecycle or optimization quality. Mitigation: return observed status and missing evidence only.
