---
name: cluster-86
description: "Skill for the Cluster_86 area of agent. 18 symbols across 2 files."
---

# Cluster_86

18 symbols | 2 files | Cohesion: 98%

## When to Use

- Working with code in `macaca/`
- Understanding how new, sender, receiver work
- Modifying cluster_86-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-ipc/src/local.rs` | new, sender, receiver, get_or_create, send (+11) |
| `macaca/crates/macaca-kernel/src/services.rs` | send, ipc_service_adapter |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `macaca/crates/macaca-ipc/src/local.rs:23`
- **`sender`** (Function) — `macaca/crates/macaca-ipc/src/local.rs:30`
- **`receiver`** (Function) — `macaca/crates/macaca-ipc/src/local.rs:37`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `macaca/crates/macaca-ipc/src/local.rs` | 23 |
| `sender` | Function | `macaca/crates/macaca-ipc/src/local.rs` | 30 |
| `receiver` | Function | `macaca/crates/macaca-ipc/src/local.rs` | 37 |
| `send` | Function | `macaca/crates/macaca-kernel/src/services.rs` | 55 |
| `ipc_service_adapter` | Function | `macaca/crates/macaca-kernel/src/services.rs` | 124 |
| `get_or_create` | Function | `macaca/crates/macaca-ipc/src/local.rs` | 45 |
| `send` | Function | `macaca/crates/macaca-ipc/src/local.rs` | 64 |
| `publish` | Function | `macaca/crates/macaca-ipc/src/local.rs` | 77 |
| `recv` | Function | `macaca/crates/macaca-ipc/src/local.rs` | 104 |
| `subscribe` | Function | `macaca/crates/macaca-ipc/src/local.rs` | 127 |
| `unsubscribe` | Function | `macaca/crates/macaca-ipc/src/local.rs` | 138 |
| `make_msg` | Function | `macaca/crates/macaca-ipc/src/local.rs` | 154 |
| `publish_and_receive` | Function | `macaca/crates/macaca-ipc/src/local.rs` | 166 |
| `send_direct_requires_to` | Function | `macaca/crates/macaca-ipc/src/local.rs` | 182 |
| `send_direct_delivers_to_agent_topic` | Function | `macaca/crates/macaca-ipc/src/local.rs` | 192 |
| `unsubscribe_stops_delivery` | Function | `macaca/crates/macaca-ipc/src/local.rs` | 209 |
| `publish_with_no_subscribers_is_ok` | Function | `macaca/crates/macaca-ipc/src/local.rs` | 224 |
| `multiple_topics` | Function | `macaca/crates/macaca-ipc/src/local.rs` | 233 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Send_direct_requires_to → Get_or_create` | intra_community | 4 |
| `Publish_and_receive → LocalSender` | intra_community | 3 |
| `Publish_and_receive → LocalReceiver` | intra_community | 3 |
| `Publish_and_receive → New` | intra_community | 3 |
| `Publish_and_receive → IpcMessage` | intra_community | 3 |
| `Publish_and_receive → Now` | cross_community | 3 |
| `Send_direct_delivers_to_agent_topic → LocalSender` | intra_community | 3 |
| `Send_direct_delivers_to_agent_topic → LocalReceiver` | intra_community | 3 |
| `Send_direct_delivers_to_agent_topic → New` | intra_community | 3 |
| `Send_direct_delivers_to_agent_topic → IpcMessage` | intra_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Executor | 2 calls |

## How to Explore

1. `gitnexus_context({name: "new"})` — see callers and callees
2. `gitnexus_query({query: "cluster_86"})` — find related execution flows
3. Read key files listed above for implementation details
