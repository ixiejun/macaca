---
name: cluster-14
description: "Skill for the Cluster_14 area of agent. 14 symbols across 3 files."
---

# Cluster_14

14 symbols | 3 files | Cohesion: 75%

## When to Use

- Working with code in `macaca/`
- Understanding how new, user, get_agent_by_name work
- Modifying cluster_14-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-web/src/agent_runner.rs` | new, get_state, load_persona, build_system_prompt, execute_agent (+5) |
| `macaca/crates/macaca-proto/src/types.rs` | user, llm_message_user_constructor |
| `macaca/crates/macaca-kernel/src/kernel.rs` | get_agent_by_name, status_tracker |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `macaca/crates/macaca-web/src/agent_runner.rs:32`
- **`user`** (Function) — `macaca/crates/macaca-proto/src/types.rs:483`
- **`get_agent_by_name`** (Function) — `macaca/crates/macaca-kernel/src/kernel.rs:91`
- **`status_tracker`** (Function) — `macaca/crates/macaca-kernel/src/kernel.rs:112`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `macaca/crates/macaca-web/src/agent_runner.rs` | 32 |
| `user` | Function | `macaca/crates/macaca-proto/src/types.rs` | 483 |
| `get_agent_by_name` | Function | `macaca/crates/macaca-kernel/src/kernel.rs` | 91 |
| `status_tracker` | Function | `macaca/crates/macaca-kernel/src/kernel.rs` | 112 |
| `get_state` | Function | `macaca/crates/macaca-web/src/agent_runner.rs` | 38 |
| `load_persona` | Function | `macaca/crates/macaca-web/src/agent_runner.rs` | 43 |
| `build_system_prompt` | Function | `macaca/crates/macaca-web/src/agent_runner.rs` | 60 |
| `execute_agent` | Function | `macaca/crates/macaca-web/src/agent_runner.rs` | 93 |
| `list_agents` | Function | `macaca/crates/macaca-web/src/agent_runner.rs` | 260 |
| `agent_exists` | Function | `macaca/crates/macaca-web/src/agent_runner.rs` | 281 |
| `execute_agent_with_events` | Function | `macaca/crates/macaca-web/src/agent_runner.rs` | 292 |
| `test_build_system_prompt_with_capabilities` | Function | `macaca/crates/macaca-web/src/agent_runner.rs` | 467 |
| `test_build_system_prompt_without_capabilities` | Function | `macaca/crates/macaca-web/src/agent_runner.rs` | 479 |
| `llm_message_user_constructor` | Function | `macaca/crates/macaca-proto/src/types.rs` | 778 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Execute_workflow_steps → User` | cross_community | 5 |
| `Execute_agent_with_events → New` | cross_community | 4 |
| `Execute_agent → New` | cross_community | 4 |
| `Execute_agent_with_events → Get_state` | intra_community | 3 |
| `Execute_agent → Get_state` | intra_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_15 | 3 calls |
| Cluster_85 | 3 calls |
| Executor | 2 calls |
| Cluster_51 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "new"})` — see callers and callees
2. `gitnexus_query({query: "cluster_14"})` — find related execution flows
3. Read key files listed above for implementation details
