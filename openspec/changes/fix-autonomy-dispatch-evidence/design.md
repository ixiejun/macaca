## Context

Macaca local autonomy runs two provider-owned flows through Runtime Host:

- Scheduler jobs become due runs, acquire leases, dispatch provider-neutral targets, and record run summaries.
- Heartbeat native cadence accepts profile wakes and bridges manifest-declared agents into Agent Execution.

The live failure showed that Heartbeat's bridge waited for long Agent Execution work inside the same supervisor loop that also advances Scheduler leases. It also showed that a completed Agent Execution service reply is not sufficient evidence that the described work produced a durable, replayable result.

## Goals / Non-Goals

- Goal: Scheduler ticks must continue while heartbeat agent work is still running.
- Goal: Successful autonomy completion must be tied to sanitized, replayable evidence from Agent Execution results.
- Goal: Logs must expose key state transitions without raw prompts or provider payloads.
- Non-goal: Add application-specific proof-file logic, workflow templates, or business-domain branches.
- Non-goal: Move Scheduler, Heartbeat, or Agent Execution ownership into Web, frontend, or application code.

## Decisions

- Use Strategy for dispatch routing and a small Specification-style evidence gate for result classification.
- Use Runtime Host as the only composition point for background heartbeat dispatch handles.
- Treat missing result evidence as retryable/failure evidence, not success, so autonomous loops do not fake completion.
- Keep evidence provider-neutral: accepted evidence is sanitized metadata/output references such as `result_evidence_ref`, `artifact_ref`, `artifact_digest`, `audit_id`, or bounded output evidence emitted by Agent Execution. `result_output_hash` is audit correlation only and must not prove autonomous task completion. Runtime Host never reads raw prompts or app-specific files to decide success.
- Use an Observer/Memento helper in the Web Agent Execution adapter to derive sanitized artifact metadata from successful generic tool events. When a caller provides `evidence.expected_artifact_path` metadata, the adapter also captures a pre-run file snapshot and emits artifact evidence only when the exact expected file is created or changed during the run. The path itself is hashed before crossing the result boundary, and the fallback does not depend on a specific writing tool.
- Render generic `evidence.*` metadata into the Agent Execution structured context so the model sees the same provider-neutral completion requirement that the adapter later validates. Runtime Host still evaluates only sanitized result metadata and does not parse prompts.

## Risks / Trade-offs

- Stricter evidence gating can turn previously green tests into retryable outcomes. Tests and fakes must provide explicit evidence to represent real completion.
- Background heartbeat dispatch means a tick reports dispatch acceptance before final agent completion. Logs must separately record spawn, completion, and failure counts so audit replay can correlate the asynchronous chain.
- Expected artifact metadata is optional and provider-neutral. It raises assurance for probe-style tasks without making Runtime Host parse `HEARTBEAT.md` content or hardcode application artifact names. The adapter uses metadata facts rather than raw artifact contents, so evidence remains compact and audit-safe.

## Verification

- Add a failing test proving `HeartbeatLane.tick_once` returns promptly while Agent Execution is still running.
- Add a failing test proving scheduled-agent dispatch does not succeed when Agent Execution returns completed without evidence.
- Add tests proving `result_output_hash` alone is not completion evidence, file-write tool evidence is accepted only when it matches the expected artifact metadata, and an unchanged expected file is not completion evidence.
- Run runtime-host autonomy tests, scheduled-agent integration boundaries, OpenSpec strict validation, and diff hygiene checks.
