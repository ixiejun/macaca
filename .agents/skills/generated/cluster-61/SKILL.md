---
name: cluster-61
description: "Skill for the Cluster_61 area of agent. 12 symbols across 1 files."
---

# Cluster_61

12 symbols | 1 files | Cohesion: 97%

## When to Use

- Working with code in `macaca/`
- Understanding how new, retrieve, delete work
- Modifying cluster_61-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-memory/src/isolated.rs` | new, retrieve, delete, auto_retrieve, make_isolated (+7) |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `macaca/crates/macaca-memory/src/isolated.rs:38`
- **`retrieve`** (Function) — `macaca/crates/macaca-memory/src/isolated.rs:110`
- **`delete`** (Function) — `macaca/crates/macaca-memory/src/isolated.rs:191`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `macaca/crates/macaca-memory/src/isolated.rs` | 38 |
| `retrieve` | Function | `macaca/crates/macaca-memory/src/isolated.rs` | 110 |
| `delete` | Function | `macaca/crates/macaca-memory/src/isolated.rs` | 191 |
| `auto_retrieve` | Function | `macaca/crates/macaca-memory/src/isolated.rs` | 207 |
| `make_isolated` | Function | `macaca/crates/macaca-memory/src/isolated.rs` | 257 |
| `make_entry` | Function | `macaca/crates/macaca-memory/src/isolated.rs` | 272 |
| `store_forces_agent_id` | Function | `macaca/crates/macaca-memory/src/isolated.rs` | 285 |
| `store_and_retrieve` | Function | `macaca/crates/macaca-memory/src/isolated.rs` | 297 |
| `different_agents_are_isolated` | Function | `macaca/crates/macaca-memory/src/isolated.rs` | 309 |
| `delete_entry` | Function | `macaca/crates/macaca-memory/src/isolated.rs` | 355 |
| `list_entries` | Function | `macaca/crates/macaca-memory/src/isolated.rs` | 364 |
| `file_directory_scoped_by_app_and_agent` | Function | `macaca/crates/macaca-memory/src/isolated.rs` | 390 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Executor | 1 calls |

## How to Explore

1. `gitnexus_context({name: "new"})` — see callers and callees
2. `gitnexus_query({query: "cluster_61"})` — find related execution flows
3. Read key files listed above for implementation details
