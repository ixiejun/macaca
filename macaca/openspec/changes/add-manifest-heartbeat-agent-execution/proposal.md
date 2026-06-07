# Change: Add Manifest-Declared Heartbeat Agent Execution

## Why

`service.heartbeat` can now produce native wake mementos, but accepted wakes do
not execute agent `HEARTBEAT.md` instructions. Macaca needs a generic,
auditable bridge that lets applications explicitly declare heartbeat
participants without making Scheduler, Web, frontend, or `HEARTBEAT.md` file
scanning own execution scope.

## What Changes

- Add application manifest heartbeat-agent declarations under
  `autonomy.heartbeat.agents`.
- Add an Application Service sanitized projection for declared heartbeat agents.
- Add a heartbeat execution intent and structured skip behavior in Agent
  Execution.
- Add a runtime-host HeartbeatLane dispatch strategy that calls Application
  Service and Agent Execution through typed commands after an accepted heartbeat
  wake.
- Preserve `service.heartbeat` as cadence/wake/memento owner only; it does not
  execute agents directly.
- Preserve Scheduler decoupling: Scheduler is not required for heartbeat agent
  execution.

## Impact

- Affected specs: `application-service`, `heartbeat-service`,
  `agent-execution-service`, `autonomous-runtime`,
  `serviceization-escape-hatches`, `serviceization-dependency-gate`
- Affected code: `macaca-proto` Application/Agent Execution DTOs,
  `macaca-app` manifest and projection, `macaca-runtime-host` autonomy
  supervisor, `macaca-web` agent execution backend, integration tests, one local
  WASM app proof fixture
