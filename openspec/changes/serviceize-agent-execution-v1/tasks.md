## 1. Contracts

- [ ] 1.1 Define provider-neutral `AgentExecutionCommand`, `AgentExecutionResult`, status, diagnostics, and structured error DTOs.
- [ ] 1.2 Define provider-neutral `AgentContextBuildCommand`, `AgentContextSnapshot`, context source, and snapshot reference DTOs.
- [ ] 1.3 Add unavailable/null providers for `service.agent_execution` and `service.agent_context`.
- [ ] 1.4 Add ServiceRuntime descriptors, health checks, lifecycle records, and command surfaces.

## 2. Context Provider

- [ ] 2.1 Extract current persona, manifest semantics, capabilities, workspace guide, skill snapshot, MCP/tool catalog, memory/context, and tool policy construction behind `service.agent_context`.
- [ ] 2.2 Preserve existing framework context behavior in the first built-in provider.
- [ ] 2.3 Emit `agent_context_built`, `skill_catalog_built`, and `skill_snapshot_created` events through EventLog and trace.
- [ ] 2.4 Add tests for persona loading, skill snapshot visibility, tool policy context, and unavailable dependency behavior.

## 3. Execution Provider

- [ ] 3.1 Wrap current framework runtime agent execution as the first built-in `service.agent_execution` provider.
- [ ] 3.2 Ensure system context is built only by `service.agent_context`.
- [ ] 3.3 Ensure application/delegate prompts are passed as user prompts, never system prompts.
- [ ] 3.4 Emit lifecycle, runtime, model, result, and failure events before live streaming.

## 4. Consumer Convergence

- [ ] 4.1 Migrate WASM `macaca:agent/delegate` to `service.agent_execution`.
- [ ] 4.2 Migrate YAML workflow agent steps to `service.agent_execution`.
- [ ] 4.3 Migrate chat main-thread execution to `service.agent_execution`.
- [ ] 4.4 Migrate task/goal worker execution to `service.agent_execution`.
- [ ] 4.5 Replace direct `AgentExecutionLauncher::launch` production use with service calls or remove it from production paths.

## 5. Boundary Gates

- [ ] 5.1 Add tests proving no production caller constructs agent runtime directly outside `service.agent_execution`.
- [ ] 5.2 Add tests proving Web shell does not own agent context semantics.
- [ ] 5.3 Add tests proving WASM and YAML delegation share the same service trace phases.
- [ ] 5.4 Add audit replay tests from application input through context build, model call, result, and UI/event output.

## 6. Verification

- [ ] 6.1 Run GitNexus impact analysis before editing implementation symbols.
- [ ] 6.2 Run `cargo fmt`.
- [ ] 6.3 Run targeted Rust tests for runtime-host, app, web, task, skill, MCP, and context areas touched by the migration.
- [ ] 6.4 Run `/api/chat/v2` manual verification for YAML, WASM, and task/goal sessions.
- [ ] 6.5 Run `openspec validate serviceize-agent-execution-v1 --strict`.
