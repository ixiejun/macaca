# Change: Refactor Route C Workspace Topology

## Why

Macaca 已经根据 Route C 完成了大量非内核能力服务化与模块化，但 Rust workspace 仍然把所有 crate 平铺在 `macaca/crates/` 下。该结构无法从文件系统层面表达 microkernel、foundation、services、runtime、application、facade、shells 和 tests 的所有权边界，容易让后续贡献者误把所有 crate 当作同级宏内核组件。

本变更把 workspace 目录拓扑调整为 Route C layer-oriented layout，同时保持 package name、Rust crate name、public API 和行为不变。

## What Changes

- 在 `macaca/crates/` 下引入 Route C layer 目录：`foundation/`、`kernel/`、`services/`、`runtime/`、`application/`、`facade/`、`shells/`、`tests/`。
- 将现有 21 个 Rust workspace crate 移动到对应 layer 目录，但不重命名 package 或 crate。
- 更新 `macaca/Cargo.toml` workspace members 和 internal workspace dependency paths。
- 新增 `macaca/crates/README.md`，说明 Route C workspace topology、old-to-new path mapping 和治理规则。
- 新增可执行 topology guard，使用 `cargo metadata --no-deps --format-version 1` 校验每个 workspace package 的 manifest path 是否位于预期 layer。
- 更新路径敏感的脚本、测试和当前治理文档，优先使用 `cargo metadata` 或 layer-aware glob，减少旧式 `crates/macaca-*` 硬编码。
- 保持依赖门禁规则继续以 crate/package name 判定架构依赖，目录 layer 只提供额外 topology 约束，不替代 forbidden edge gate。

## Impact

- Affected specs: `workspace-topology`
- Affected code:
  - `macaca/Cargo.toml`
  - `macaca/crates/**/Cargo.toml` manifest paths through directory moves
  - `macaca/crates/tests/macaca-integration-tests/tests/**` topology/path-sensitive tests
  - `scripts/check-cli-consumer-migration.sh`
  - `scripts/check-web-cli-thin-shell.sh`
  - any active script with hardcoded old crate paths
- Affected docs:
  - `macaca/crates/README.md`
  - `macaca/docs/agent-os-microkernel-boundaries.md`
  - `macaca/docs/route-c-architecture-governance.md`
  - `macaca/docs/route-c-serviceization-allowlist.md` if topology clarification is needed
- Affected validation:
  - `openspec validate refactor-route-c-workspace-topology --strict`
  - `cargo metadata --no-deps --format-version 1`
  - `cargo fmt --all --check`
  - `cargo check --workspace`
  - `cargo test -p macaca-integration-tests route_c_dependency_boundaries`
  - `cargo test -p macaca-integration-tests --test route_c_baseline`
  - `cargo test -p macaca-integration-tests route_c_workspace_topology`

## Governance Alignment

- Follows `docs/openharmony-microkernel-architecture-for-macaca-agent-os.md`: filesystem topology should reflect microkernel, system services, application framework, SDK facade, optional/module/runtime ownership, and thin shell boundaries.
- Follows `macaca/docs/agent-os-microkernel-boundaries.md`: kernel owns invariants; services own replaceable capabilities; Web/CLI are presentation shells; optional modules remain optional.
- Follows `macaca/docs/route-c-architecture-governance.md`: architecture constraints should be executable, auditable, and represented by tests or documented allowlist entries.
- Uses Layers, Facade, Bridge, Adapter, Registry, Specification, and Memento patterns.

## Non-Goals

- Do not rename package names, Rust crate names, modules, service IDs, command names, routes, CLI commands, or wire formats.
- Do not introduce new crates such as `macaca-store`, `macaca-payment`, `macaca-web3`, `macaca-evm`, or `macaca-ui`.
- Do not split `macaca-runtime-host` internals in this topology-only change.
- Do not remove deprecated compatibility anchors.
- Do not delete Route C allowlist rows merely because crate paths move.
- Do not change `/api/chat/v2`, SSE trace, session replay, task board, application lifecycle, provider behavior, Store/Entitlement, Payment/A2A, Web3/EVM, Web UI, or CLI behavior.
- Do not bulk rewrite historical OpenSpec/research documents that mention old paths unless the path is an active command or executable check.
