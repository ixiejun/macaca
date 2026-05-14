---
name: executor
description: "Skill for the Executor area of agent. 162 symbols across 24 files."
---

# Executor

162 symbols | 24 files | Cohesion: 78%

## When to Use

- Working with code in `macaca/`
- Understanding how assign, fail, manifest work
- Modifying executor-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-kernel/src/executor/app_executor.rs` | delegate_task, route_and_delegate, execute_agent, default, new (+19) |
| `macaca/crates/macaca-kernel/src/executor/fork_manager.rs` | complete_fork, fail_fork, subscribe_to_hooks, start_fork, new (+14) |
| `macaca/crates/macaca-kernel/src/executor/worker.rs` | new, application_id, router, event_bus, command_channel (+10) |
| `macaca/crates/macaca-kernel/src/executor/bus.rs` | new, now, default, test_publish_subscribe, event_type (+9) |
| `macaca/crates/macaca-kernel/src/executor/callback.rs` | new, with_coordinator_channel, queue, notify_completed, notify_failed (+9) |
| `macaca/crates/macaca-kernel/src/executor/queue.rs` | fail, status, new, partial_cmp, cmp (+7) |
| `macaca/crates/macaca-kernel/src/executor/router.rs` | new, route, route_by_capability, calculate_score, generate_reasoning (+7) |
| `macaca/crates/macaca-memory/src/vector.rs` | new, ensure_collection, upsert, search, delete (+6) |
| `macaca/crates/macaca-web/src/routes.rs` | diagnose_llm_error, new, on_task_started, get_all, post_chat (+6) |
| `macaca/crates/macaca-kernel/src/logging.rs` | with_trace_id, with_fork_id, log_state_transition, log_operation_complete, elapsed (+1) |

## Entry Points

Start here when exploring this area:

- **`assign`** (Function) — `macaca/crates/macaca-task/src/tracker.rs:46`
- **`fail`** (Function) — `macaca/crates/macaca-task/src/tracker.rs:118`
- **`manifest`** (Function) — `macaca/crates/macaca-sdk/src/builder.rs:125`
- **`register`** (Function) — `macaca/crates/macaca-kernel/src/status.rs:23`
- **`update_state`** (Function) — `macaca/crates/macaca-kernel/src/status.rs:43`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `assign` | Function | `macaca/crates/macaca-task/src/tracker.rs` | 46 |
| `fail` | Function | `macaca/crates/macaca-task/src/tracker.rs` | 118 |
| `manifest` | Function | `macaca/crates/macaca-sdk/src/builder.rs` | 125 |
| `register` | Function | `macaca/crates/macaca-kernel/src/status.rs` | 23 |
| `update_state` | Function | `macaca/crates/macaca-kernel/src/status.rs` | 43 |
| `fail` | Function | `macaca/crates/macaca-kernel/src/executor/queue.rs` | 168 |
| `complete_fork` | Function | `macaca/crates/macaca-kernel/src/executor/fork_manager.rs` | 500 |
| `fail_fork` | Function | `macaca/crates/macaca-kernel/src/executor/fork_manager.rs` | 513 |
| `new` | Function | `macaca/crates/macaca-kernel/src/executor/bus.rs` | 17 |
| `now` | Function | `macaca/crates/macaca-kernel/src/executor/bus.rs` | 27 |
| `delegate_task` | Function | `macaca/crates/macaca-kernel/src/executor/app_executor.rs` | 202 |
| `route_and_delegate` | Function | `macaca/crates/macaca-kernel/src/executor/app_executor.rs` | 293 |
| `new` | Function | `macaca/crates/macaca-memory/src/vector.rs` | 68 |
| `ensure_collection` | Function | `macaca/crates/macaca-memory/src/vector.rs` | 78 |
| `status` | Function | `macaca/crates/macaca-kernel/src/executor/queue.rs` | 229 |
| `subscribe_to_hooks` | Function | `macaca/crates/macaca-kernel/src/executor/fork_manager.rs` | 645 |
| `new` | Function | `macaca/crates/macaca-kernel/src/executor/app_executor.rs` | 125 |
| `shutdown` | Function | `macaca/crates/macaca-kernel/src/executor/app_executor.rs` | 379 |
| `register_application` | Function | `macaca/crates/macaca-kernel/src/executor/app_executor.rs` | 728 |
| `register_application_with_config` | Function | `macaca/crates/macaca-kernel/src/executor/app_executor.rs` | 743 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Main → Now` | cross_community | 7 |
| `Start_server → Validate` | cross_community | 7 |
| `Execute_workflow_steps → StoredTurn` | cross_community | 7 |
| `Start_server → AgentConfig` | cross_community | 6 |
| `Start_server → CapabilityDef` | cross_community | 6 |
| `Run → Calculate_score` | cross_community | 6 |
| `Run → Generate_reasoning` | cross_community | 6 |
| `Execute_workflow_steps → StoredSession` | cross_community | 6 |
| `Execute_workflow_steps → SessionMeta` | cross_community | 6 |
| `Register_from_file_yaml → AgentManifest` | cross_community | 6 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Tests | 6 calls |
| Cluster_14 | 4 calls |
| Cluster_15 | 3 calls |
| Cluster_51 | 2 calls |
| Cluster_8 | 1 calls |
| Cluster_45 | 1 calls |
| Cluster_131 | 1 calls |
| Cluster_5 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "assign"})` — see callers and callees
2. `gitnexus_query({query: "executor"})` — find related execution flows
3. Read key files listed above for implementation details
