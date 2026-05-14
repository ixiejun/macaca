---
name: cluster-89
description: "Skill for the Cluster_89 area of agent. 11 symbols across 1 files."
---

# Cluster_89

11 symbols | 1 files | Cohesion: 97%

## When to Use

- Working with code in `macaca/`
- Understanding how new, unregister work
- Modifying cluster_89-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-kernel/src/registry.rs` | new, unregister, make_manifest, mock_agent, register_and_count (+6) |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `macaca/crates/macaca-kernel/src/registry.rs:28`
- **`unregister`** (Function) — `macaca/crates/macaca-kernel/src/registry.rs:67`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `macaca/crates/macaca-kernel/src/registry.rs` | 28 |
| `unregister` | Function | `macaca/crates/macaca-kernel/src/registry.rs` | 67 |
| `make_manifest` | Function | `macaca/crates/macaca-kernel/src/registry.rs` | 131 |
| `mock_agent` | Function | `macaca/crates/macaca-kernel/src/registry.rs` | 176 |
| `register_and_count` | Function | `macaca/crates/macaca-kernel/src/registry.rs` | 184 |
| `get_returns_manifest` | Function | `macaca/crates/macaca-kernel/src/registry.rs` | 193 |
| `unregister_removes_agent` | Function | `macaca/crates/macaca-kernel/src/registry.rs` | 203 |
| `unregister_missing_is_error` | Function | `macaca/crates/macaca-kernel/src/registry.rs` | 212 |
| `max_agents_enforced` | Function | `macaca/crates/macaca-kernel/src/registry.rs` | 220 |
| `duplicate_registration_is_error` | Function | `macaca/crates/macaca-kernel/src/registry.rs` | 230 |
| `list_returns_all_manifests` | Function | `macaca/crates/macaca-kernel/src/registry.rs` | 242 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Executor | 1 calls |

## How to Explore

1. `gitnexus_context({name: "new"})` — see callers and callees
2. `gitnexus_query({query: "cluster_89"})` — find related execution flows
3. Read key files listed above for implementation details
