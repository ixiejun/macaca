---
name: builtin
description: "Skill for the Builtin area of agent. 20 symbols across 2 files."
---

# Builtin

20 symbols | 2 files | Cohesion: 100%

## When to Use

- Working with code in `macaca/`
- Understanding how new, with_timeout, new work
- Modifying builtin-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/crates/macaca-driver/src/builtin/shell_driver.rs` | new, with_timeout, default, manifest, initialize (+6) |
| `macaca/crates/macaca-driver/src/builtin/filesystem_driver.rs` | new, default, manifest, initialize, tools (+4) |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `macaca/crates/macaca-driver/src/builtin/shell_driver.rs:21`
- **`with_timeout`** (Function) — `macaca/crates/macaca-driver/src/builtin/shell_driver.rs:36`
- **`new`** (Function) — `macaca/crates/macaca-driver/src/builtin/filesystem_driver.rs:19`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `macaca/crates/macaca-driver/src/builtin/shell_driver.rs` | 21 |
| `with_timeout` | Function | `macaca/crates/macaca-driver/src/builtin/shell_driver.rs` | 36 |
| `new` | Function | `macaca/crates/macaca-driver/src/builtin/filesystem_driver.rs` | 19 |
| `default` | Function | `macaca/crates/macaca-driver/src/builtin/shell_driver.rs` | 43 |
| `manifest` | Function | `macaca/crates/macaca-driver/src/builtin/shell_driver.rs` | 50 |
| `initialize` | Function | `macaca/crates/macaca-driver/src/builtin/shell_driver.rs` | 54 |
| `tools` | Function | `macaca/crates/macaca-driver/src/builtin/shell_driver.rs` | 59 |
| `shutdown` | Function | `macaca/crates/macaca-driver/src/builtin/shell_driver.rs` | 70 |
| `shell_driver_manifest` | Function | `macaca/crates/macaca-driver/src/builtin/shell_driver.rs` | 80 |
| `shell_driver_tools` | Function | `macaca/crates/macaca-driver/src/builtin/shell_driver.rs` | 89 |
| `shell_driver_lifecycle` | Function | `macaca/crates/macaca-driver/src/builtin/shell_driver.rs` | 97 |
| `shell_driver_custom_timeout` | Function | `macaca/crates/macaca-driver/src/builtin/shell_driver.rs` | 105 |
| `default` | Function | `macaca/crates/macaca-driver/src/builtin/filesystem_driver.rs` | 34 |
| `manifest` | Function | `macaca/crates/macaca-driver/src/builtin/filesystem_driver.rs` | 41 |
| `initialize` | Function | `macaca/crates/macaca-driver/src/builtin/filesystem_driver.rs` | 45 |
| `tools` | Function | `macaca/crates/macaca-driver/src/builtin/filesystem_driver.rs` | 50 |
| `shutdown` | Function | `macaca/crates/macaca-driver/src/builtin/filesystem_driver.rs` | 58 |
| `filesystem_driver_manifest` | Function | `macaca/crates/macaca-driver/src/builtin/filesystem_driver.rs` | 68 |
| `filesystem_driver_tools` | Function | `macaca/crates/macaca-driver/src/builtin/filesystem_driver.rs` | 78 |
| `filesystem_driver_lifecycle` | Function | `macaca/crates/macaca-driver/src/builtin/filesystem_driver.rs` | 88 |

## How to Explore

1. `gitnexus_context({name: "new"})` — see callers and callees
2. `gitnexus_query({query: "builtin"})` — find related execution flows
3. Read key files listed above for implementation details
