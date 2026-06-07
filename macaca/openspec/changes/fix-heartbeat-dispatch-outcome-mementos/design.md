## Context
`service.heartbeat` owns native cadence, gates, run state, and replayable
mementos. Runtime Host owns the bridge from accepted heartbeat wakes to Agent
Execution. The live test showed the current split records wake success before
the asynchronous Agent Execution outcome is known.

## Goals / Non-Goals
- Goal: heartbeat run history must reflect the terminal outcome of delegated
  heartbeat agent execution.
- Goal: completion metadata must be bounded, sanitized, provider-neutral, and
  traceable.
- Non-goal: make Heartbeat execute agents, inspect prompts, or encode
  application-specific workflows.
- Non-goal: make Scheduler responsible for heartbeat agent execution.

## Decisions
- Use the Command pattern by adding `heartbeat.run.complete` as a typed Heartbeat
  Service command.
- Use the Observer pattern by letting Runtime Host observe Agent Execution
  results and report a sanitized outcome to Heartbeat.
- Use the Memento pattern by storing the final dispatch outcome in
  `HeartbeatRunSummary.metadata`.
- Keep `tick_native_profiles_once` responsible only for wake acceptance and
  entering the dispatch boundary. It must not mark accepted runs succeeded before
  dispatch completion is observed.

## Risks / Trade-offs
- Existing tests that treated native wake acceptance as completion must be
  updated to the stricter state model.
- Legacy application-scoped wakes may dispatch multiple declarations. The
  aggregate outcome is failed if any enabled dispatch fails, succeeded if at
  least one dispatch succeeds and none fail, and skipped when no eligible
  dispatch exists.

## Migration Plan
The command is additive. Existing wake, query, and profile APIs remain
compatible. Older providers that do not implement the command return structured
unsupported/unavailable behavior through the service adapter.
