# Workflow execution inventory — task 2.4.1

> GitNexus impact memo (non-blocking): `WebAgentRunner::execute_via_agent_service` — HIGH;
> `WebApplicationOrchestrationBackend::delegate_agent` — MEDIUM.

## Scope

Inventory of YAML workflow execution landing points referenced by task 2.4.1:
`macaca-app/src/workflow.rs` and `macaca-web/src/agent_runner.rs`.

## Findings

### `macaca-app/src/workflow.rs` — prompt/validation only

| Responsibility | Executes agents? | Notes |
|----------------|------------------|-------|
| `WorkflowPromptStrategy` / `DefaultWorkflowPromptStrategy` | No | Renders step prompts from manifest templates |
| `validate_workflow_dependencies` | No | Detects dependency cycles in workflow graph |
| `resolve_entrypoint_workflow` | No | Resolves configured entry workflow name |

**Conclusion:** `macaca-app` owns application semantics (prompt rendering, validation).
It does **not** execute workflow steps. Execution is delegated to the host shell.

### `macaca-web/src/agent_runner.rs` — pre-2.4.2 gap

| Component | Role | Pre-convergence path |
|-----------|------|----------------------|
| `WebAgentRunner` | Implements kernel `AgentRunner` for `ApplicationExecutorRegistry` | Executor schedules step → `AgentRunner` trait |
| `execute_via_agent_service` | Workflow step execution adapter | **Direct** `service.agent_execution` (`YamlWorkflowStep`) |

**Gap (yaml-B chain in `audit-replay-baseline.md`):**

```
ApplicationExecutor worker
  → WebAgentRunner (kernel AgentRunner trait)
  → service.agent_execution   ← bypassed Application Service
```

### WASM reference path (target pattern)

```
WASM host import / Application Service
  → application.agent.delegate
  → WebApplicationOrchestrationBackend
  → service.agent_execution (WasmDelegate intent)
```

### Target unified path (task 2.4.2)

```
ApplicationExecutor worker
  → WebAgentRunner
  → service.application / application.agent.delegate
  → WebApplicationOrchestrationBackend
  → service.agent_execution (YamlWorkflowStep intent)
```

YAML workflow and WASM delegation then share the same Application ABI entry
before converging on the single `service.agent_execution` provider.

## Consumers of `WebAgentRunner`

- `ApplicationExecutorRegistry` — YAML manifest workflow step dispatch
- Kernel `AgentRunner` trait — stable executor-facing contract (scheduling shell)

The executor remains a scheduling shell; execution semantics must not be
reintroduced in kernel code.

## Metadata contract

`ApplicationAgentDelegateCommand.metadata["execution_intent"]` selects intent:

| Wire value | `AgentExecutionIntent` |
|------------|------------------------|
| `yaml_workflow_step` | `YamlWorkflowStep` |
| `wasm_delegate` (default) | `WasmDelegate` |

Defined in `macaca-proto::agent_execution_service` as provider-neutral labels.
