---
name: cluster-55
description: "Skill for the Cluster_55 area of agent. 15 symbols across 2 files."
---

# Cluster_55

15 symbols | 2 files | Cohesion: 92%

## When to Use

- Working with code in `macaca/`
- Understanding how resolved_permission_level, from_config, with_id work
- Modifying cluster_55-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-sdk/src/builder.rs` | from_config, with_id, with_model, with_prompt, build (+9) |
| `macaca/crates/macaca-sdk/src/config.rs` | resolved_permission_level |

## Entry Points

Start here when exploring this area:

- **`resolved_permission_level`** (Function) — `macaca/crates/macaca-sdk/src/config.rs:145`
- **`from_config`** (Function) — `macaca/crates/macaca-sdk/src/builder.rs:23`
- **`with_id`** (Function) — `macaca/crates/macaca-sdk/src/builder.rs:28`
- **`with_model`** (Function) — `macaca/crates/macaca-sdk/src/builder.rs:34`
- **`with_prompt`** (Function) — `macaca/crates/macaca-sdk/src/builder.rs:40`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `resolved_permission_level` | Function | `macaca/crates/macaca-sdk/src/config.rs` | 145 |
| `from_config` | Function | `macaca/crates/macaca-sdk/src/builder.rs` | 23 |
| `with_id` | Function | `macaca/crates/macaca-sdk/src/builder.rs` | 28 |
| `with_model` | Function | `macaca/crates/macaca-sdk/src/builder.rs` | 34 |
| `with_prompt` | Function | `macaca/crates/macaca-sdk/src/builder.rs` | 40 |
| `build` | Function | `macaca/crates/macaca-sdk/src/builder.rs` | 46 |
| `sample_config` | Function | `macaca/crates/macaca-sdk/src/builder.rs` | 215 |
| `builder_builds_agent` | Function | `macaca/crates/macaca-sdk/src/builder.rs` | 230 |
| `builder_with_id_override` | Function | `macaca/crates/macaca-sdk/src/builder.rs` | 241 |
| `builder_with_model_override` | Function | `macaca/crates/macaca-sdk/src/builder.rs` | 251 |
| `builder_with_prompt_override` | Function | `macaca/crates/macaca-sdk/src/builder.rs` | 260 |
| `build_with_manifest_produces_both` | Function | `macaca/crates/macaca-sdk/src/builder.rs` | 270 |
| `build_invalid_config_fails` | Function | `macaca/crates/macaca-sdk/src/builder.rs` | 280 |
| `declarative_agent_run_calls_llm` | Function | `macaca/crates/macaca-sdk/src/builder.rs` | 288 |
| `declarative_agent_empty_prompt_errors` | Function | `macaca/crates/macaca-sdk/src/builder.rs` | 302 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Remove_stopped_app → Capability` | cross_community | 6 |
| `Start_and_list_app → Capability` | cross_community | 6 |
| `Stop_already_stopped_is_ok → Capability` | cross_community | 6 |
| `Remove_running_app_fails → Capability` | cross_community | 6 |
| `App_agents_returns_ids → Capability` | cross_community | 6 |
| `Native_app_no_agents → Capability` | cross_community | 6 |
| `Native_app_no_agents → Permission` | cross_community | 6 |
| `Native_app_no_agents → Resolved_permission_level` | cross_community | 6 |
| `Start_duplicate_app_fails → Capability` | cross_community | 6 |
| `Wasm_app_not_supported → Capability` | cross_community | 6 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_54 | 1 calls |
| Cluster_53 | 1 calls |
| Tests | 1 calls |

## How to Explore

1. `gitnexus_context({name: "resolved_permission_level"})` — see callers and callees
2. `gitnexus_query({query: "cluster_55"})` — find related execution flows
3. Read key files listed above for implementation details
