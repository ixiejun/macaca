# Change: Serviceize unified agent execution

## Why

Macaca OS currently has multiple production paths that can start agent work. This violates the microkernel and serviceization constitutions and lets some paths, especially WASM `agent.delegate`, bypass persona, skill snapshot, tool policy, context construction, trace, and audit behavior used by YAML applications.

## What Changes

- Add `service.agent_execution` as the only production boundary for starting agent work.
- Add `service.agent_context` as the owner of trusted agent context construction.
- Require YAML, WASM, chat, task, goal, worker, SDK, and future application adapters to produce typed agent execution commands instead of owning execution semantics.
- Move existing framework runner behavior behind service providers instead of Web shell semantic ownership.
- Deprecate direct production use of executor fast paths and thin launchers that bypass service context construction.

## Impact

- Affected specs: `agent-execution-service`, `agent-context-service`, `application-orchestration`
- Affected code areas: `macaca-runtime-host` ServiceRuntime providers, `macaca-web` framework runner adapters, WASM host import bridge, Application Service orchestration backend, task/goal worker execution, YAML workflow execution, chat session orchestration
- Governance impact: enforces `macaca-os-architecture-governance.md`, `macaca-os-microkernel-boundaries.md`, and `macaca-os-serviceization-allowlist.md`
