# Audit Replay Baseline — Pre-convergence Multi-path Inventory

Captured: 2026-06-07 (iteration 7, task 0.3)

Purpose: record the **pre-convergence** agent execution evidence chains for YAML and WASM
sessions before task 2.6 coordination-patch deletion.  Gate: convergence target is **1**
distinct `service.agent_execution` chain per session replay (task 2.7.1).

Method: static code-path inventory + `system.service_audit` replay contract.  Live
`/api/chat/v2` runs require provider credentials and are deferred to route-c regression;
this document is the authoritative **pre-delete** memento for task 2.6.

## Replay command surface

| Command | Service | Purpose |
|---------|---------|---------|
| `service.audit.replay.session` | `system.service_audit` | Ordered service-call evidence by `session_id` |
| `service.audit.replay.trace` | `system.service_audit` | Ordered service-call evidence by `trace_id` |

## YAML session (`/api/chat/v2` — manifest/workflow path)

Observed execution chains (**3 distinct**, pre-convergence):

| Chain ID | Entry surface | Primary services touched | Coordination marker |
|----------|---------------|--------------------------|---------------------|
| `yaml-A` | `chat_orchestrator` → `FrameworkRunner` | `service.agent_execution` (ComposedAgentExecutionBackend), `service.agent_context`, `service.llm` | `legacy_chat_main_thread_goal_pause` / local pause channels |
| `yaml-B` | `agent_runner` workflow steps (`macaca-app/workflow.rs`) | `Kernel::execute_agent` port → `service.agent_execution` adapter | `suppress_executor_lifecycle` on some branches |
| `yaml-C` | `delegate_task` tool (orchestration) | `service.agent_execution` (ServiceDelegatedTaskDispatcher) + kernel `ForkManager` suspend/resume | `fork_to_session` shell mapping + hook_consumer legacy channel |

**Distinct chain count: 3** (target after P1: 1)

## WASM session (`/api/chat/v2` — application execution protocol)

Observed execution chains (**3 distinct**, pre-convergence):

| Chain ID | Entry surface | Primary services touched | Coordination marker |
|----------|---------------|--------------------------|---------------------|
| `wasm-A` | `application_execution_hosted` start/control | `service.application_execution` → hosted provider | `authoritative` / `non_authoritative` / `legacy_unmarked` graph_owner |
| `wasm-B` | WASM host import bridge agent run | `service.agent_execution` via import bridge | `execution.graph_owner` compatibility labels |
| `wasm-C` | Worker/plan loops via web `loop_manager` | `service.task`, direct executor registry hooks | `graph_owner` on task assignment |

**Distinct chain count: 3** (target after P1: 1)

## Fork-Join delegate path (cross-cutting)

After iteration 7 partial 2.3.2 wiring:

```
delegate_task (orchestration_tools)
  → ExecutionControlForkJoinCoordinator.register_parent_fork_wait  [service.execution_control]
  → ServiceDelegatedTaskDispatcher.dispatch                    [service.agent_execution]
  → kernel ForkManager suspend                                 [kernel — pending P3 eviction]
hook_consumer (ForkValidated / DelegateFailed)
  → ExecutionControlForkJoinCoordinator.deliver_parent_fork_resume [service.execution_control]
  → legacy ActiveSession.resume_tx adapter                     [shell compat — pending 2.3.4]
```

This adds **execution_control** evidence but does not yet reduce chain count to 1 because
YAML/WASM primary run paths remain parallel (chains A/B/C above).

## Convergence gate (task 2.6 prerequisite)

Do **not** delete coordination patches until replay shows:

- YAML session: **1** chain dominated by `service.agent_execution` provider
- WASM session: **1** chain dominated by `service.application_execution` + same agent execution provider
- No production writes to: `graph_owner`, `authoritative`, `legacy_unmarked`,
  `suppress_executor_lifecycle`, `legacy_chat_main_thread_goal_pause`

## Verification commands (when live server available)

```bash
# After one YAML chat session (note session_id from response):
curl -s localhost:3001/api/... # or SDK client
# Replay:
# service.audit.replay.session { session_id }

# After one WASM application execution session:
# service.audit.replay.session { session_id }
```

Integration proxy (no LLM): `cargo test -p macaca-runtime-host service_call_audit` and
`cargo test -p macaca-runtime-host execution_control_fork_join`.
