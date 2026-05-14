---
name: cluster-63
description: "Skill for the Cluster_63 area of agent. 13 symbols across 1 files."
---

# Cluster_63

13 symbols | 1 files | Cohesion: 93%

## When to Use

- Working with code in `macaca/`
- Understanding how new work
- Modifying cluster_63-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-memory/src/file.rs` | new, retrieve, delete, make_entry, store_and_get (+8) |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `macaca/crates/macaca-memory/src/file.rs:17`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `macaca/crates/macaca-memory/src/file.rs` | 17 |
| `retrieve` | Function | `macaca/crates/macaca-memory/src/file.rs` | 67 |
| `delete` | Function | `macaca/crates/macaca-memory/src/file.rs` | 92 |
| `make_entry` | Function | `macaca/crates/macaca-memory/src/file.rs` | 121 |
| `store_and_get` | Function | `macaca/crates/macaca-memory/src/file.rs` | 134 |
| `get_missing_returns_none` | Function | `macaca/crates/macaca-memory/src/file.rs` | 144 |
| `retrieve_by_substring` | Function | `macaca/crates/macaca-memory/src/file.rs` | 152 |
| `retrieve_case_insensitive` | Function | `macaca/crates/macaca-memory/src/file.rs` | 164 |
| `delete_entry` | Function | `macaca/crates/macaca-memory/src/file.rs` | 174 |
| `delete_missing_is_ok` | Function | `macaca/crates/macaca-memory/src/file.rs` | 183 |
| `list_by_agent` | Function | `macaca/crates/macaca-memory/src/file.rs` | 190 |
| `retrieve_limit_respected` | Function | `macaca/crates/macaca-memory/src/file.rs` | 206 |
| `persists_across_instances` | Function | `macaca/crates/macaca-memory/src/file.rs` | 217 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_64 | 2 calls |
| Executor | 1 calls |

## How to Explore

1. `gitnexus_context({name: "new"})` — see callers and callees
2. `gitnexus_query({query: "cluster_63"})` — find related execution flows
3. Read key files listed above for implementation details
