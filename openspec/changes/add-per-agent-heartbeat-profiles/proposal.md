# Change: Add Per-Agent Heartbeat Profiles

## Why

Manifest-declared heartbeat agents currently share one application-scoped native
Heartbeat profile, so users cannot assign independent heartbeat cadence or gate
policy to each agent.

## What Changes

- Register one native Heartbeat profile per manifest-declared heartbeat agent.
- Extend sanitized Application Service heartbeat declarations with concrete
  native profile id, wake scope key, cadence, and cooldown fields.
- Extend Heartbeat profile summaries and updates so operators can edit fixed
  interval and cooldown independently.
- Update Web/frontend Heartbeat Operations to aggregate per-agent profiles and
  runs without becoming the owner of heartbeat semantics.

## Impact

- Affected specs: `heartbeat-service`, `autonomous-runtime`,
  `web-cli-thin-shell-v0`
- Affected code: Application manifest model/projection, Heartbeat DTOs and local
  provider gates, runtime-host autonomy supervisor/dispatch, Web heartbeat
  operations routes, frontend heartbeat operations panel
- Risk: public DTO expansion and runtime registration path touch shared service
  boundaries; changes are additive and tested with compatibility fallback.
