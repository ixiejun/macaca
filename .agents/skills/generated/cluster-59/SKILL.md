---
name: cluster-59
description: "Skill for the Cluster_59 area of agent. 12 symbols across 1 files."
---

# Cluster_59

12 symbols | 1 files | Cohesion: 90%

## When to Use

- Working with code in `macaca/`
- Understanding how new, evict_expired work
- Modifying cluster_59-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-memory/src/session.rs` | new, evict_expired, retrieve, delete, make_entry (+7) |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `macaca/crates/macaca-memory/src/session.rs:24`
- **`evict_expired`** (Function) — `macaca/crates/macaca-memory/src/session.rs:53`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `macaca/crates/macaca-memory/src/session.rs` | 24 |
| `evict_expired` | Function | `macaca/crates/macaca-memory/src/session.rs` | 53 |
| `retrieve` | Function | `macaca/crates/macaca-memory/src/session.rs` | 79 |
| `delete` | Function | `macaca/crates/macaca-memory/src/session.rs` | 111 |
| `make_entry` | Function | `macaca/crates/macaca-memory/src/session.rs` | 142 |
| `store_and_get` | Function | `macaca/crates/macaca-memory/src/session.rs` | 155 |
| `retrieve_by_substring` | Function | `macaca/crates/macaca-memory/src/session.rs` | 164 |
| `retrieve_case_insensitive` | Function | `macaca/crates/macaca-memory/src/session.rs` | 175 |
| `delete_entry` | Function | `macaca/crates/macaca-memory/src/session.rs` | 184 |
| `ttl_eviction` | Function | `macaca/crates/macaca-memory/src/session.rs` | 192 |
| `list_by_agent` | Function | `macaca/crates/macaca-memory/src/session.rs` | 202 |
| `retrieve_limit_respected` | Function | `macaca/crates/macaca-memory/src/session.rs` | 218 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Executor | 4 calls |

## How to Explore

1. `gitnexus_context({name: "new"})` — see callers and callees
2. `gitnexus_query({query: "cluster_59"})` — find related execution flows
3. Read key files listed above for implementation details
