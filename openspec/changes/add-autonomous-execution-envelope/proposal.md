# Change: Add Autonomous Execution Envelope

## Why

Heartbeat and scheduled-agent-task runs currently pass natural-language work to
Agent Execution as prompt/context. Real runtime proof showed a heartbeat agent
could accept the wake and still produce business analysis instead of the exact
requested artifact, so wake success and task completion are not separated
strongly enough.

## What Changes

- Add a provider-neutral execution envelope to Agent Execution commands.
- Compile heartbeat and scheduled-agent-task dispatches into that envelope from
  source instruction, metadata, and generic evidence requirements.
- Render the envelope as the highest-priority delegated execution contract.
- Preserve generic service boundaries: no application, workflow, provider,
  model, or business-domain branches are introduced.

## Impact

- Affected specs: autonomous-runtime
- Affected code: `macaca-proto` Agent Execution DTOs,
  `macaca-runtime-host` autonomy dispatch strategies, and `macaca-web` Agent
  Execution backend prompt/evidence rendering.
