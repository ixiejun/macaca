# CLI Decoupling Inventory (P4 §5.1)

> Recorded during iteration 37. Confirms `macaca-cli` is a thin presentation shell
> with no kernel/gateway/web internals linkage.

## Dependency surface

```text
cargo tree -e normal -p macaca-cli --depth 1
→ macaca-proto, macaca-sdk (+ tokio/clap/reqwest/tracing/serde/uuid/…)
```

No `macaca-kernel`, `macaca-gateway`, `macaca-web`, `macaca-tools`, or
`macaca-runtime-host` production dependencies in `macaca-cli/Cargo.toml`.

## Command handler inventory

| Command | Entry | Runtime seam | Notes |
|---------|-------|--------------|-------|
| `run` | `commands.rs::execute_run_kernel` | `SystemFacade` + `ServiceInspectionCommand` | Warns operator to start concrete runtimes via service hosts or `macaca web`; no `KernelBuilder` |
| `status` | `commands.rs::execute_show_status` | `SystemFacade::status_snapshot` | `StaticSystemStatusDataSource` Null Object path |
| `agents` | `commands.rs::execute_list_agents` | `SystemFacade::status_snapshot` | Diagnostic listing via SDK boundary |
| `web` | `command_handlers.rs::WebCommandHandler` | `WebServerProcessLauncher` subprocess | Spawns `macaca-web-server` binary; **no** `macaca_web::` imports |
| Skill ops | `skill_operations::execute_*` | SDK `SystemSkillClient` + optional live HTTP adapter | Public Web REST facade when `app_id` supplied; Null Object otherwise |

## Static scan results (production `macaca-cli/src`)

| Forbidden symbol / pattern | Hits |
|----------------------------|------|
| `Kernel` / `KernelBuilder` | 0 |
| `GatewayBuilder` | 0 |
| `LlmProvider` | 0 |
| `macaca_web::` | 0 |
| `macaca_runtime_host::` | 0 |

## Contract tests

- `command_handlers::tests::web_command_uses_only_public_server_start_seam`
- `skill_operations::tests::cli_skill_operations_do_not_import_runtime_or_web`

## Allowlist status

- Route C dependency allowlist: **no macaca-cli rows**
- OS layer filesize allowlist: **no macaca-cli rows** (after §4.3.16 split)

## Replacement path summary

Presentation shell → `macaca-sdk` focused clients → live service hosts started
separately (`macaca web` subprocess or external deployment). CLI never composes
providers or owns execution semantics.
