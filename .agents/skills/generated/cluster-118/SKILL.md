---
name: cluster-118
description: "Skill for the Cluster_118 area of agent. 11 symbols across 2 files."
---

# Cluster_118

11 symbols | 2 files | Cohesion: 92%

## When to Use

- Working with code in `macaca/`
- Understanding how config, new, with_model work
- Modifying cluster_118-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-driver-Codex/src/config.rs` | default_claude_bin, default_timeout, new, with_model, dangerously_skip_permissions (+4) |
| `macaca/crates/macaca-driver-Codex/src/driver.rs` | config, driver_with_custom_config |

## Entry Points

Start here when exploring this area:

- **`config`** (Function) — `macaca/crates/macaca-driver-Codex/src/driver.rs:57`
- **`new`** (Function) — `macaca/crates/macaca-driver-Codex/src/config.rs:66`
- **`with_model`** (Function) — `macaca/crates/macaca-driver-Codex/src/config.rs:80`
- **`dangerously_skip_permissions`** (Function) — `macaca/crates/macaca-driver-Codex/src/config.rs:86`
- **`with_timeout`** (Function) — `macaca/crates/macaca-driver-Codex/src/config.rs:92`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `config` | Function | `macaca/crates/macaca-driver-Codex/src/driver.rs` | 57 |
| `new` | Function | `macaca/crates/macaca-driver-Codex/src/config.rs` | 66 |
| `with_model` | Function | `macaca/crates/macaca-driver-Codex/src/config.rs` | 80 |
| `dangerously_skip_permissions` | Function | `macaca/crates/macaca-driver-Codex/src/config.rs` | 86 |
| `with_timeout` | Function | `macaca/crates/macaca-driver-Codex/src/config.rs` | 92 |
| `driver_with_custom_config` | Function | `macaca/crates/macaca-driver-Codex/src/driver.rs` | 159 |
| `default_claude_bin` | Function | `macaca/crates/macaca-driver-Codex/src/config.rs` | 56 |
| `default_timeout` | Function | `macaca/crates/macaca-driver-Codex/src/config.rs` | 60 |
| `default_config` | Function | `macaca/crates/macaca-driver-Codex/src/config.rs` | 103 |
| `builder_methods` | Function | `macaca/crates/macaca-driver-Codex/src/config.rs` | 113 |
| `serialize_roundtrip` | Function | `macaca/crates/macaca-driver-Codex/src/config.rs` | 128 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Tests | 1 calls |

## How to Explore

1. `gitnexus_context({name: "config"})` — see callers and callees
2. `gitnexus_query({query: "cluster_118"})` — find related execution flows
3. Read key files listed above for implementation details
