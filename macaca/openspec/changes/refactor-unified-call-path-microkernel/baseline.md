# Baseline — refactor-unified-call-path-microkernel

Captured: 2026-06-07 (iteration 1)

## OpenSpec

- Change id: `refactor-unified-call-path-microkernel`
- Delta requirements: 21 (validate with `npx openspec validate refactor-unified-call-path-microkernel --strict`)

## Route C dependency gate

- Production edges visited: **100** (dev edges skipped)
- Allowlist rows: **10** (3 kernel provider-compat + 7 web thin-shell)
- Gate: `cargo test -p macaca-integration-tests --test route_c_dependency_boundaries route_c_dependency_boundaries_reject_unallowlisted_forbidden_edges` → **PASS**

### Allowlist snapshot

| from | to | owner track | phase |
|------|-----|-------------|-------|
| macaca-kernel | macaca-driver | kernel provider compatibility | S6 |
| macaca-kernel | macaca-gateway | kernel provider compatibility | S8 |
| macaca-kernel | macaca-skill | kernel provider compatibility | S6 |
| macaca-web | macaca-driver | Web thin shell | S6 |
| macaca-web | macaca-llm | Web thin shell | S5 |
| macaca-web | macaca-memory | Web thin shell | S5 |
| macaca-web | macaca-persist | Web thin shell | S1/S12 |
| macaca-web | macaca-skill | Web thin shell | S6 |
| macaca-web | macaca-task | Web thin shell | S4 |
| macaca-web | macaca-tools | Web thin shell | S6 |

Note: kernel production `Cargo.toml` currently lists only `macaca-proto`, `macaca-agent`, `macaca-ipc`; driver/gateway/skill are **dev-dependencies** only. Allowlist kernel rows are migration mementos until removed in P2.6.

## Dependency trees (`cargo tree -e normal --depth 1`)

### macaca-kernel

```
macaca-kernel
├── macaca-agent
├── macaca-ipc
├── macaca-proto
└── (stdlib + reqwest/tokio/serde/…)
```

Target: **macaca-proto + macaca-ipc only** (remove macaca-agent after AgentExecutionPort contract moves to proto).

### macaca-web

Direct workspace deps: agent, app, context, driver, framework, kernel, llm, memory, persist, proto, runtime, runtime-host, sdk, skill, task, tools.

Target: **macaca-sdk + macaca-proto** (+ HTTP stack only).

### macaca-cli

```
macaca-cli → macaca-proto, macaca-sdk (+ clap/reqwest/…)
```

Already near target (no macaca-web direct dep in production tree).

### macaca-persist

```
macaca-persist → macaca-context, macaca-proto
```

Target: remove `macaca-context` edge (P2.7).

## Escape hatch gate

- Test: `cargo test -p macaca-integration-tests serviceization_escape_hatches_reject_new_production_references` → **PASS** after P0 token extension (1.1.1–1.1.7).

## Audit replay (0.3 — captured iteration 7)

See `audit-replay-baseline.md`. Pre-convergence inventory: **YAML 3 chains**, **WASM 3 chains** (static code-path audit). Target post-P1: **1 chain** per session before task 2.6 coordination-patch deletion.

## Design open questions (0.4)

| Q | Decision |
|---|----------|
| Q1 web3/evm | Reuse existing runtime-host optional service providers; evict kernel modules in P2 |
| Q2 persist→context | Extract shared persistence DTOs to `macaca-proto`; invert dependency so context depends on persist contract |
| Q3 Fork-Join | `service.execution_control` owns pause/resume/checkpoint; task service emits graph events |

## GitNexus impact memo (non-blocking)

HIGH/CRITICAL symbols noted per design.md D8: `Kernel::execute_agent`, `executor::*`, `AppState` fields, `application_execution_hosted::*`. Record only; do not block merges per user directive.
