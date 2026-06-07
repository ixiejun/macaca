# Change: Fix Heartbeat Dispatch Outcome Mementos

## Why
Heartbeat run history currently marks accepted native wakes as succeeded before
Runtime Host finishes Agent Execution dispatch. A heartbeat run can therefore
look successful even when the delegated agent times out or fails completion
evidence checks.

## What Changes
- Add a provider-neutral Heartbeat run completion command for runtime-owned
  dispatch outcomes.
- Delay terminal heartbeat run state until the dispatch observer reports
  `Succeeded`, `Failed`, or `Skipped`.
- Record only sanitized dispatch metadata such as completion policy, source kind,
  evidence key, status, and stable reason code.

## Impact
- Affected specs: `heartbeat-service`, `autonomous-runtime`
- Affected code: Heartbeat DTOs, local Heartbeat provider, runtime-host Heartbeat
  service adapter, HeartbeatLane, heartbeat agent dispatch strategy, focused tests
