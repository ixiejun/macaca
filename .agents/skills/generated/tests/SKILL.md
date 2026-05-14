---
name: tests
description: "Skill for the Tests area of agent. 144 symbols across 22 files."
---

# Tests

144 symbols | 22 files | Cohesion: 79%

## When to Use

- Working with code in `macaca/`
- Understanding how new, create, start work
- Modifying tests-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-task/src/tracker.rs` | new, create, start, complete, status (+16) |
| `macaca/crates/macaca-integration-tests/tests/fullstack_autodev.rs` | make_kernel, app_manifest_loads, app_starts_with_three_agents, e2e_fullstack_autodev_architect_executes, agent_skills_activate (+12) |
| `macaca/crates/macaca-skill/src/catalog.rs` | new, load_from_directory, catalog, catalog_prompt, activate (+7) |
| `macaca/crates/macaca-integration-tests/tests/gateway_pipeline.rs` | new, handle, telegram_config, discord_config, register_and_start_stop_adapters (+4) |
| `macaca/crates/macaca-gateway/src/gateway.rs` | register_adapter, start_all, stop_all, start, stop (+4) |
| `macaca/crates/macaca-driver-Codex/src/driver.rs` | new, manifest, initialize, health_check, shutdown (+4) |
| `macaca/crates/macaca-integration-tests/tests/app_declarative.rs` | make_kernel, inline_manifest, start_declarative_app_registers_agents, stop_app_unregisters_agents, multiple_apps_coexist (+3) |
| `macaca/crates/macaca-integration-tests/tests/task_lifecycle.rs` | make_request, make_result, tracker_and_queue_full_lifecycle, queue_respects_priority_ordering, list_tasks_by_status_across_lifecycle (+2) |
| `macaca/crates/macaca-kernel/tests/e2e_auto_programming.rs` | make_kernel, code_gen_config, planner_config, test_ac1_auto_programming_flow, test_ac1_agent_receives_prompt (+2) |
| `macaca/crates/macaca-integration-tests/tests/kernel_lifecycle.rs` | make_kernel, sample_agent_config, declarative_agent_full_lifecycle, register_multiple_agents, execute_nonexistent_agent_returns_not_found (+2) |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `macaca/crates/macaca-task/src/tracker.rs:17`
- **`create`** (Function) — `macaca/crates/macaca-task/src/tracker.rs:24`
- **`start`** (Function) — `macaca/crates/macaca-task/src/tracker.rs:70`
- **`complete`** (Function) — `macaca/crates/macaca-task/src/tracker.rs:93`
- **`status`** (Function) — `macaca/crates/macaca-task/src/tracker.rs:140`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `macaca/crates/macaca-task/src/tracker.rs` | 17 |
| `create` | Function | `macaca/crates/macaca-task/src/tracker.rs` | 24 |
| `start` | Function | `macaca/crates/macaca-task/src/tracker.rs` | 70 |
| `complete` | Function | `macaca/crates/macaca-task/src/tracker.rs` | 93 |
| `status` | Function | `macaca/crates/macaca-task/src/tracker.rs` | 140 |
| `get_result` | Function | `macaca/crates/macaca-task/src/tracker.rs` | 149 |
| `list_by_status` | Function | `macaca/crates/macaca-task/src/tracker.rs` | 153 |
| `list_by_agent` | Function | `macaca/crates/macaca-task/src/tracker.rs` | 158 |
| `get_app_agents` | Function | `macaca/crates/macaca-web/src/routes.rs` | 257 |
| `list_for_agents` | Function | `macaca/crates/macaca-kernel/src/status.rs` | 108 |
| `list_agent_statuses_for` | Function | `macaca/crates/macaca-kernel/src/kernel.rs` | 132 |
| `start_app_from_file` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 29 |
| `remove_app` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 112 |
| `list_apps` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 130 |
| `app_agents` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 138 |
| `app_status` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 147 |
| `load_manifest` | Function | `macaca/crates/macaca-app/src/loader.rs` | 15 |
| `build_with_manifest` | Function | `macaca/crates/macaca-sdk/src/builder.rs` | 88 |
| `agents_read` | Function | `macaca/crates/macaca-kernel/src/registry.rs` | 110 |
| `register_agent` | Function | `macaca/crates/macaca-kernel/src/kernel.rs` | 40 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Main → Now` | cross_community | 7 |
| `Start_server → Validate` | cross_community | 7 |
| `Start_server → AgentConfig` | cross_community | 6 |
| `Start_server → CapabilityDef` | cross_community | 6 |
| `Reload_apps → Validate_manifest` | cross_community | 6 |
| `Remove_stopped_app → Capability` | cross_community | 6 |
| `Start_and_list_app → Capability` | cross_community | 6 |
| `Stop_already_stopped_is_ok → Capability` | cross_community | 6 |
| `Remove_running_app_fails → Capability` | cross_community | 6 |
| `App_agents_returns_ids → Capability` | cross_community | 6 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_129 | 12 calls |
| Executor | 10 calls |
| Cluster_51 | 5 calls |
| Cluster_53 | 4 calls |
| Cluster_55 | 2 calls |
| Cluster_85 | 2 calls |
| Cluster_30 | 1 calls |
| Cluster_34 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "new"})` — see callers and callees
2. `gitnexus_query({query: "tests"})` — find related execution flows
3. Read key files listed above for implementation details
