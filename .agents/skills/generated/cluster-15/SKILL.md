---
name: cluster-15
description: "Skill for the Cluster_15 area of agent. 21 symbols across 4 files."
---

# Cluster_15

21 symbols | 4 files | Cohesion: 86%

## When to Use

- Working with code in `macaca/`
- Understanding how run, run_with_events, check_and_consume_resume work
- Modifying cluster_15-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-runtime/src/agentic_loop.rs` | run, run_with_events, execute_tool_call_with_events, execute_tool_call, accumulate_usage (+4) |
| `macaca/crates/macaca-proto/src/types.rs` | assistant_with_tool_calls, thinking, tool_call_with_id, tool_result_with_error, completed (+1) |
| `macaca/crates/macaca-runtime/src/permission.rs` | check_tool_permission, restricted_permission, restricted_allows_listed_tool, restricted_denies_unlisted_tool |
| `macaca/crates/macaca-tools/src/tool.rs` | get_tool, to_definitions |

## Entry Points

Start here when exploring this area:

- **`run`** (Function) — `macaca/crates/macaca-runtime/src/agentic_loop.rs:75`
- **`run_with_events`** (Function) — `macaca/crates/macaca-runtime/src/agentic_loop.rs:186`
- **`check_and_consume_resume`** (Function) — `macaca/crates/macaca-runtime/src/agentic_loop.rs:552`
- **`run_with_pause`** (Function) — `macaca/crates/macaca-runtime/src/agentic_loop.rs:564`
- **`assistant_with_tool_calls`** (Function) — `macaca/crates/macaca-proto/src/types.rs:503`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `run` | Function | `macaca/crates/macaca-runtime/src/agentic_loop.rs` | 75 |
| `run_with_events` | Function | `macaca/crates/macaca-runtime/src/agentic_loop.rs` | 186 |
| `check_and_consume_resume` | Function | `macaca/crates/macaca-runtime/src/agentic_loop.rs` | 552 |
| `run_with_pause` | Function | `macaca/crates/macaca-runtime/src/agentic_loop.rs` | 564 |
| `assistant_with_tool_calls` | Function | `macaca/crates/macaca-proto/src/types.rs` | 503 |
| `thinking` | Function | `macaca/crates/macaca-proto/src/types.rs` | 639 |
| `tool_call_with_id` | Function | `macaca/crates/macaca-proto/src/types.rs` | 664 |
| `tool_result_with_error` | Function | `macaca/crates/macaca-proto/src/types.rs` | 682 |
| `completed` | Function | `macaca/crates/macaca-proto/src/types.rs` | 696 |
| `get_tool` | Function | `macaca/crates/macaca-tools/src/tool.rs` | 49 |
| `to_definitions` | Function | `macaca/crates/macaca-tools/src/tool.rs` | 54 |
| `check_tool_permission` | Function | `macaca/crates/macaca-runtime/src/permission.rs` | 24 |
| `restricted_permission` | Function | `macaca/crates/macaca-runtime/src/permission.rs` | 60 |
| `restricted_allows_listed_tool` | Function | `macaca/crates/macaca-runtime/src/permission.rs` | 78 |
| `restricted_denies_unlisted_tool` | Function | `macaca/crates/macaca-runtime/src/permission.rs` | 87 |
| `execute_tool_call_with_events` | Function | `macaca/crates/macaca-runtime/src/agentic_loop.rs` | 325 |
| `execute_tool_call` | Function | `macaca/crates/macaca-runtime/src/agentic_loop.rs` | 436 |
| `accumulate_usage` | Function | `macaca/crates/macaca-runtime/src/agentic_loop.rs` | 479 |
| `chat` | Function | `macaca/crates/macaca-runtime/src/agentic_loop.rs` | 781 |
| `accumulate_usage_test` | Function | `macaca/crates/macaca-runtime/src/agentic_loop.rs` | 1111 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Run_with_pause → ToolDefinition` | intra_community | 3 |
| `Run → ToolDefinition` | intra_community | 3 |
| `Run → LlmResponse` | intra_community | 3 |
| `Run → New` | cross_community | 3 |
| `Run → Default` | cross_community | 3 |
| `Run_with_events → ToolDefinition` | intra_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_27 | 4 calls |
| Cluster_14 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "run"})` — see callers and callees
2. `gitnexus_query({query: "cluster_15"})` — find related execution flows
3. Read key files listed above for implementation details
