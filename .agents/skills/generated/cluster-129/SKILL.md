---
name: cluster-129
description: "Skill for the Cluster_129 area of agent. 16 symbols across 1 files."
---

# Cluster_129

16 symbols | 1 files | Cohesion: 82%

## When to Use

- Working with code in `macaca/`
- Understanding how new, start_app, stop_app work
- Modifying cluster_129-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-app/src/runtime.rs` | new, start_app, stop_app, find_by_name, default (+11) |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `macaca/crates/macaca-app/src/runtime.rs:22`
- **`start_app`** (Function) — `macaca/crates/macaca-app/src/runtime.rs:45`
- **`stop_app`** (Function) — `macaca/crates/macaca-app/src/runtime.rs:90`
- **`find_by_name`** (Function) — `macaca/crates/macaca-app/src/runtime.rs:156`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 22 |
| `start_app` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 45 |
| `stop_app` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 90 |
| `find_by_name` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 156 |
| `default` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 170 |
| `make_kernel` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 216 |
| `inline_manifest` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 226 |
| `start_and_list_app` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 255 |
| `start_duplicate_app_fails` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 272 |
| `stop_already_stopped_is_ok` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 302 |
| `remove_stopped_app` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 314 |
| `remove_running_app_fails` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 326 |
| `app_agents_returns_ids` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 340 |
| `stop_nonexistent_app_fails` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 357 |
| `wasm_app_not_supported` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 366 |
| `native_app_no_agents` | Function | `macaca/crates/macaca-app/src/runtime.rs` | 387 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Start_server → Validate` | cross_community | 7 |
| `Start_server → AgentConfig` | cross_community | 6 |
| `Start_server → CapabilityDef` | cross_community | 6 |
| `Remove_stopped_app → Validate` | cross_community | 6 |
| `Remove_stopped_app → Capability` | cross_community | 6 |
| `Start_and_list_app → Validate` | cross_community | 6 |
| `Start_and_list_app → Capability` | cross_community | 6 |
| `Stop_already_stopped_is_ok → Validate` | cross_community | 6 |
| `Stop_already_stopped_is_ok → Capability` | cross_community | 6 |
| `Remove_running_app_fails → Validate` | cross_community | 6 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Tests | 7 calls |
| Cluster_134 | 1 calls |
| Cluster_49 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "new"})` — see callers and callees
2. `gitnexus_query({query: "cluster_129"})` — find related execution flows
3. Read key files listed above for implementation details
