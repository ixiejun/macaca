# Change: Fix autonomy dispatch evidence and heartbeat isolation

## Why

Live validation showed two autonomy defects: heartbeat agent execution can hold the supervisor loop while long LLM/tool work runs, preventing Scheduler runs from being leased, and Agent Execution completion can be treated as final success without a replayable result evidence reference.

## What Changes

- Keep Heartbeat native cadence and Scheduler due-run dispatch independent by moving heartbeat agent execution into bounded background dispatch from the Heartbeat lane.
- Require scheduled-agent-task and heartbeat-agent success classification to depend on decoded Agent Execution results plus sanitized evidence metadata, not only service-call success or `agent.execute completed`.
- Treat model output hashes as audit correlation only, and derive completion evidence from durable artifact/audit refs such as sanitized generic file-write tool evidence.
- Preserve OS boundaries: Scheduler still owns time/leases/run state, Heartbeat still owns native cadence, Agent Execution still owns model/tool execution, and Runtime Host owns only provider-neutral dispatch strategy and audit correlation.

## Impact

- Affected specs: autonomous-runtime, serviceization-escape-hatches
- Affected code: `macaca-runtime-host` autonomy supervisor lanes and dispatch strategies, focused runtime-host tests, OpenSpec validation
