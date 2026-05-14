---
name: cluster-131
description: "Skill for the Cluster_131 area of agent. 10 symbols across 1 files."
---

# Cluster_131

10 symbols | 1 files | Cohesion: 85%

## When to Use

- Working with code in `macaca/`
- Understanding how new, with_dirs, user_apps_dir work
- Modifying cluster_131-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-app/src/registry.rs` | new, with_dirs, user_apps_dir, discover_apps, find_app_dir (+5) |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `macaca/crates/macaca-app/src/registry.rs:47`
- **`with_dirs`** (Function) — `macaca/crates/macaca-app/src/registry.rs:56`
- **`user_apps_dir`** (Function) — `macaca/crates/macaca-app/src/registry.rs:70`
- **`discover_apps`** (Function) — `macaca/crates/macaca-app/src/registry.rs:81`
- **`find_app_dir`** (Function) — `macaca/crates/macaca-app/src/registry.rs:172`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `macaca/crates/macaca-app/src/registry.rs` | 47 |
| `with_dirs` | Function | `macaca/crates/macaca-app/src/registry.rs` | 56 |
| `user_apps_dir` | Function | `macaca/crates/macaca-app/src/registry.rs` | 70 |
| `discover_apps` | Function | `macaca/crates/macaca-app/src/registry.rs` | 81 |
| `find_app_dir` | Function | `macaca/crates/macaca-app/src/registry.rs` | 172 |
| `default` | Function | `macaca/crates/macaca-app/src/registry.rs` | 217 |
| `registry_new_is_empty` | Function | `macaca/crates/macaca-app/src/registry.rs` | 227 |
| `find_app_dir_returns_none_for_nonexistent` | Function | `macaca/crates/macaca-app/src/registry.rs` | 233 |
| `discover_apps_from_empty_dir` | Function | `macaca/crates/macaca-app/src/registry.rs` | 239 |
| `discover_apps_from_dir_with_app` | Function | `macaca/crates/macaca-app/src/registry.rs` | 251 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Reload_apps → Validate_manifest` | cross_community | 6 |
| `Start_server → Validate_manifest` | cross_community | 5 |
| `Reload_apps → New` | cross_community | 4 |
| `Reload_apps → User_apps_dir` | cross_community | 4 |
| `Reload_apps → DiscoveredApp` | cross_community | 4 |
| `Start_server → New` | cross_community | 3 |
| `Start_server → User_apps_dir` | cross_community | 3 |
| `Start_server → DiscoveredApp` | cross_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Tests | 1 calls |
| Cluster_5 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "new"})` — see callers and callees
2. `gitnexus_query({query: "cluster_131"})` — find related execution flows
3. Read key files listed above for implementation details
