---
name: cluster-18
description: "Skill for the Cluster_18 area of agent. 13 symbols across 2 files."
---

# Cluster_18

13 symbols | 2 files | Cohesion: 100%

## When to Use

- Working with code in `macaca/`
- Understanding how cost_for, default_pricing, new work
- Modifying cluster_18-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-llm/src/cost.rs` | cost_for, default_pricing, new, record, total_cost_usd (+6) |
| `macaca/crates/macaca-tools/src/builtin.rs` | execute, file_read_write_roundtrip |

## Entry Points

Start here when exploring this area:

- **`cost_for`** (Function) — `macaca/crates/macaca-llm/src/cost.rs:11`
- **`default_pricing`** (Function) — `macaca/crates/macaca-llm/src/cost.rs:19`
- **`new`** (Function) — `macaca/crates/macaca-llm/src/cost.rs:55`
- **`record`** (Function) — `macaca/crates/macaca-llm/src/cost.rs:60`
- **`total_cost_usd`** (Function) — `macaca/crates/macaca-llm/src/cost.rs:84`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `cost_for` | Function | `macaca/crates/macaca-llm/src/cost.rs` | 11 |
| `default_pricing` | Function | `macaca/crates/macaca-llm/src/cost.rs` | 19 |
| `new` | Function | `macaca/crates/macaca-llm/src/cost.rs` | 55 |
| `record` | Function | `macaca/crates/macaca-llm/src/cost.rs` | 60 |
| `total_cost_usd` | Function | `macaca/crates/macaca-llm/src/cost.rs` | 84 |
| `reset` | Function | `macaca/crates/macaca-llm/src/cost.rs` | 93 |
| `execute` | Function | `macaca/crates/macaca-tools/src/builtin.rs` | 40 |
| `file_read_write_roundtrip` | Function | `macaca/crates/macaca-tools/src/builtin.rs` | 224 |
| `usage` | Function | `macaca/crates/macaca-llm/src/cost.rs` | 103 |
| `cost_accumulates` | Function | `macaca/crates/macaca-llm/src/cost.rs` | 108 |
| `reset_clears_all` | Function | `macaca/crates/macaca-llm/src/cost.rs` | 127 |
| `unknown_model_zero_cost` | Function | `macaca/crates/macaca-llm/src/cost.rs` | 137 |
| `tracker_is_clone_shared` | Function | `macaca/crates/macaca-llm/src/cost.rs` | 145 |

## How to Explore

1. `gitnexus_context({name: "cost_for"})` — see callers and callees
2. `gitnexus_query({query: "cluster_18"})` — find related execution flows
3. Read key files listed above for implementation details
