---
name: cluster-66
description: "Skill for the Cluster_66 area of agent. 10 symbols across 2 files."
---

# Cluster_66

10 symbols | 2 files | Cohesion: 85%

## When to Use

- Working with code in `macaca/`
- Understanding how new, new, connect work
- Modifying cluster_66-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-mcp/src/driver.rs` | new, initialize, tools, shutdown, mcp_driver_lifecycle |
| `macaca/crates/macaca-mcp/src/client.rs` | new, connect, disconnect, register_tools, client_lifecycle |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `macaca/crates/macaca-mcp/src/driver.rs:31`
- **`new`** (Function) — `macaca/crates/macaca-mcp/src/client.rs:82`
- **`connect`** (Function) — `macaca/crates/macaca-mcp/src/client.rs:114`
- **`disconnect`** (Function) — `macaca/crates/macaca-mcp/src/client.rs:121`
- **`register_tools`** (Function) — `macaca/crates/macaca-mcp/src/client.rs:173`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `macaca/crates/macaca-mcp/src/driver.rs` | 31 |
| `new` | Function | `macaca/crates/macaca-mcp/src/client.rs` | 82 |
| `connect` | Function | `macaca/crates/macaca-mcp/src/client.rs` | 114 |
| `disconnect` | Function | `macaca/crates/macaca-mcp/src/client.rs` | 121 |
| `register_tools` | Function | `macaca/crates/macaca-mcp/src/client.rs` | 173 |
| `initialize` | Function | `macaca/crates/macaca-mcp/src/driver.rs` | 56 |
| `tools` | Function | `macaca/crates/macaca-mcp/src/driver.rs` | 79 |
| `shutdown` | Function | `macaca/crates/macaca-mcp/src/driver.rs` | 112 |
| `mcp_driver_lifecycle` | Function | `macaca/crates/macaca-mcp/src/driver.rs` | 125 |
| `client_lifecycle` | Function | `macaca/crates/macaca-mcp/src/client.rs` | 202 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Mcp_driver_lifecycle → DriverManifest` | intra_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_68 | 3 calls |

## How to Explore

1. `gitnexus_context({name: "new"})` — see callers and callees
2. `gitnexus_query({query: "cluster_66"})` — find related execution flows
3. Read key files listed above for implementation details
