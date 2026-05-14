---
name: cluster-32
description: "Skill for the Cluster_32 area of agent. 11 symbols across 1 files."
---

# Cluster_32

11 symbols | 1 files | Cohesion: 95%

## When to Use

- Working with code in `macaca/`
- Understanding how with_central_store, register_client, list_central_skills work
- Modifying cluster_32-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-skill/src/provisioner.rs` | with_central_store, register_client, list_central_skills, provision_all_for_client, provision_skill (+6) |

## Entry Points

Start here when exploring this area:

- **`with_central_store`** (Function) — `macaca/crates/macaca-skill/src/provisioner.rs:81`
- **`register_client`** (Function) — `macaca/crates/macaca-skill/src/provisioner.rs:94`
- **`list_central_skills`** (Function) — `macaca/crates/macaca-skill/src/provisioner.rs:116`
- **`provision_all_for_client`** (Function) — `macaca/crates/macaca-skill/src/provisioner.rs:145`
- **`provision_skill`** (Function) — `macaca/crates/macaca-skill/src/provisioner.rs:192`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `with_central_store` | Function | `macaca/crates/macaca-skill/src/provisioner.rs` | 81 |
| `register_client` | Function | `macaca/crates/macaca-skill/src/provisioner.rs` | 94 |
| `list_central_skills` | Function | `macaca/crates/macaca-skill/src/provisioner.rs` | 116 |
| `provision_all_for_client` | Function | `macaca/crates/macaca-skill/src/provisioner.rs` | 145 |
| `provision_skill` | Function | `macaca/crates/macaca-skill/src/provisioner.rs` | 192 |
| `provision_for_app` | Function | `macaca/crates/macaca-skill/src/provisioner.rs` | 233 |
| `copy_skill_dir` | Function | `macaca/crates/macaca-skill/src/provisioner.rs` | 258 |
| `provision_specific_skill` | Function | `macaca/crates/macaca-skill/src/provisioner.rs` | 339 |
| `provision_missing_skill` | Function | `macaca/crates/macaca-skill/src/provisioner.rs` | 374 |
| `provision_unknown_client` | Function | `macaca/crates/macaca-skill/src/provisioner.rs` | 392 |
| `provision_with_subdirectories` | Function | `macaca/crates/macaca-skill/src/provisioner.rs` | 431 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Provision_all_for_client → Home_dir` | cross_community | 6 |
| `Provision_specific_skill → Home_dir` | cross_community | 5 |
| `Provision_missing_skill → Home_dir` | cross_community | 5 |
| `Provision_with_subdirectories → Home_dir` | cross_community | 5 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_31 | 2 calls |

## How to Explore

1. `gitnexus_context({name: "with_central_store"})` — see callers and callees
2. `gitnexus_query({query: "cluster_32"})` — find related execution flows
3. Read key files listed above for implementation details
