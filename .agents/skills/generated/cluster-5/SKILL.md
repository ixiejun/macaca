---
name: cluster-5
description: "Skill for the Cluster_5 area of agent. 9 symbols across 3 files."
---

# Cluster_5

9 symbols | 3 files | Cohesion: 82%

## When to Use

- Working with code in `macaca/`
- Understanding how get_status, get_apps, get_app work
- Modifying cluster_5-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-web/src/routes.rs` | get_status, get_apps, get_app, reload_apps |
| `macaca/crates/macaca-app/src/registry.rs` | get_app_by_name, get_default_app, clear, reload |
| `macaca/crates/macaca-kernel/src/kernel.rs` | agent_count |

## Entry Points

Start here when exploring this area:

- **`get_status`** (Function) — `macaca/crates/macaca-web/src/routes.rs:105`
- **`get_apps`** (Function) — `macaca/crates/macaca-web/src/routes.rs:131`
- **`get_app`** (Function) — `macaca/crates/macaca-web/src/routes.rs:166`
- **`reload_apps`** (Function) — `macaca/crates/macaca-web/src/routes.rs:403`
- **`agent_count`** (Function) — `macaca/crates/macaca-kernel/src/kernel.rs:97`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `get_status` | Function | `macaca/crates/macaca-web/src/routes.rs` | 105 |
| `get_apps` | Function | `macaca/crates/macaca-web/src/routes.rs` | 131 |
| `get_app` | Function | `macaca/crates/macaca-web/src/routes.rs` | 166 |
| `reload_apps` | Function | `macaca/crates/macaca-web/src/routes.rs` | 403 |
| `agent_count` | Function | `macaca/crates/macaca-kernel/src/kernel.rs` | 97 |
| `get_app_by_name` | Function | `macaca/crates/macaca-app/src/registry.rs` | 156 |
| `get_default_app` | Function | `macaca/crates/macaca-app/src/registry.rs` | 166 |
| `clear` | Function | `macaca/crates/macaca-app/src/registry.rs` | 204 |
| `reload` | Function | `macaca/crates/macaca-app/src/registry.rs` | 210 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Reload_apps → Validate_manifest` | cross_community | 6 |
| `Reload_apps → New` | cross_community | 4 |
| `Reload_apps → User_apps_dir` | cross_community | 4 |
| `Reload_apps → DiscoveredApp` | cross_community | 4 |
| `Main → Agent_count` | cross_community | 3 |
| `Reload_apps → Clear` | intra_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_131 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "get_status"})` — see callers and callees
2. `gitnexus_query({query: "cluster_5"})` — find related execution flows
3. Read key files listed above for implementation details
