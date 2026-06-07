## 1. Contracts

- [x] 1.1 Define provider-neutral `AgentExecutionCommand`, `AgentExecutionResult`, status, diagnostics, and structured error DTOs.
- [x] 1.2 Define provider-neutral `AgentContextBuildCommand`, `AgentContextSnapshot`, context source, and snapshot reference DTOs.
- [x] 1.3 Add unavailable/null providers for `service.agent_execution` and `service.agent_context`.
- [x] 1.4 Add ServiceRuntime descriptors, health checks, lifecycle records, and command surfaces.

## 2. Context Provider

- [x] 2.1 Extract current persona, manifest semantics, capabilities, workspace guide, skill snapshot, MCP/tool catalog, memory/context, and tool policy construction behind `service.agent_context`.
- [x] 2.2 Preserve existing framework context behavior in the first built-in provider.
- [x] 2.3 Emit `agent_context_built`, `skill_catalog_built`, and `skill_snapshot_created` events through EventLog and trace.
- [x] 2.4 Add tests for persona loading, skill snapshot visibility, tool policy context, and unavailable dependency behavior.

## 3. Execution Provider

- [x] 3.1 Wrap current framework runtime agent execution as the first built-in `service.agent_execution` provider.
- [x] 3.2 Ensure system context is built only by `service.agent_context`.
- [x] 3.3 Ensure application/delegate prompts are passed as user prompts, never system prompts.
- [x] 3.4 Emit lifecycle, runtime, model, result, and failure events before live streaming.

## 4. Consumer Convergence

- [x] 4.1 Migrate WASM `macaca:agent/delegate` to `service.agent_execution`.
- [x] 4.2 Migrate YAML workflow agent steps to `service.agent_execution`.
- [x] 4.3 Migrate chat main-thread execution to `service.agent_execution`.
- [x] 4.4 Migrate task/goal worker execution to `service.agent_execution`.
- [x] 4.5 Replace direct `AgentExecutionLauncher::launch` production use with service calls or remove it from production paths.

## 5. Boundary Gates

- [x] 5.1 Add tests proving no production caller constructs agent runtime directly outside `service.agent_execution`.
- [x] 5.2 Add tests proving Web shell does not own agent context semantics.
- [x] 5.3 Add tests proving WASM and YAML delegation share the same service trace phases.
- [x] 5.4 Add audit replay tests from application input through context build, model call, result, and UI/event output.

## 6. Verification

- [x] 6.1 Run GitNexus impact analysis before editing implementation symbols.
- [x] 6.2 Run `cargo fmt`.
- [x] 6.3 Run targeted Rust tests for runtime-host, app, web, task, skill, MCP, and context areas touched by the migration.
- [x] 6.4 Run `/api/chat/v2` manual verification for YAML, WASM, and task/goal sessions.
  - YAML chat returned `OK` with `execution_intent: ChatMainThread` on `:3011`.
  - WASM delegation emitted `execution_intent: WasmDelegate` plus delegate start/tool/complete events on `:3011`.
  - Task/goal session `manual-goal-debug-6-4` completed end-to-end on `:3011`: planner decomposed goal `aa19d5d7-ca5c-426e-a669-a7574f852a00`, worker `architect` claimed task `03040010-0e9b-4dac-8c7a-ae6958030a36` through `execution_intent: TaskWorker`, submitted review, planner approved, and the goal reached `Completed`.
- [x] 6.5 Run `openspec validate serviceize-agent-execution-v1 --strict`.
