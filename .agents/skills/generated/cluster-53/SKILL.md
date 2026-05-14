---
name: cluster-53
description: "Skill for the Cluster_53 area of agent. 10 symbols across 1 files."
---

# Cluster_53

10 symbols | 1 files | Cohesion: 69%

## When to Use

- Working with code in `macaca/`
- Understanding how from_yaml work
- Modifying cluster_53-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-sdk/src/config.rs` | from_yaml, parse_yaml_minimal, parse_yaml_full, empty_name_fails_validation, invalid_permission_level_fails (+5) |

## Entry Points

Start here when exploring this area:

- **`from_yaml`** (Function) — `macaca/crates/macaca-sdk/src/config.rs:72`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `from_yaml` | Function | `macaca/crates/macaca-sdk/src/config.rs` | 72 |
| `parse_yaml_minimal` | Function | `macaca/crates/macaca-sdk/src/config.rs` | 158 |
| `parse_yaml_full` | Function | `macaca/crates/macaca-sdk/src/config.rs` | 175 |
| `empty_name_fails_validation` | Function | `macaca/crates/macaca-sdk/src/config.rs` | 226 |
| `invalid_permission_level_fails` | Function | `macaca/crates/macaca-sdk/src/config.rs` | 235 |
| `empty_capability_name_fails` | Function | `macaca/crates/macaca-sdk/src/config.rs` | 245 |
| `temperature_out_of_range_fails` | Function | `macaca/crates/macaca-sdk/src/config.rs` | 256 |
| `resolved_permission_defaults_to_user` | Function | `macaca/crates/macaca-sdk/src/config.rs` | 277 |
| `persona_dir_optional` | Function | `macaca/crates/macaca-sdk/src/config.rs` | 286 |
| `persona_dir_defaults_to_none` | Function | `macaca/crates/macaca-sdk/src/config.rs` | 296 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Start_server → Validate` | cross_community | 7 |
| `Remove_stopped_app → Validate` | cross_community | 6 |
| `Start_and_list_app → Validate` | cross_community | 6 |
| `Stop_already_stopped_is_ok → Validate` | cross_community | 6 |
| `Remove_running_app_fails → Validate` | cross_community | 6 |
| `App_agents_returns_ids → Validate` | cross_community | 6 |
| `Native_app_no_agents → Validate` | cross_community | 6 |
| `Start_duplicate_app_fails → Validate` | cross_community | 6 |
| `Wasm_app_not_supported → Validate` | cross_community | 6 |
| `Builder_with_id_override → Validate` | cross_community | 4 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_54 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "from_yaml"})` — see callers and callees
2. `gitnexus_query({query: "cluster_53"})` — find related execution flows
3. Read key files listed above for implementation details
