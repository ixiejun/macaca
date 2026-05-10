## 1. Preparation

- [x] 1.1 Read `docs/superpowers/plans/2026-05-11-route-c-workspace-topology-refactor-plan.md`.
- [x] 1.2 Read `docs/superpowers/plans/2026-05-11-route-c-workspace-topology-refactor-brainstorm.md`.
- [x] 1.3 Read `docs/openharmony-microkernel-architecture-for-macaca-agent-os.md`, `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-architecture-governance.md`, `macaca/docs/route-c-serviceization-allowlist.md`, and `macaca/docs/design_patterns.md`.
- [x] 1.4 Confirm there are exactly 21 current workspace packages before the move using `cargo metadata --no-deps --format-version 1`.
- [x] 1.5 Run GitNexus impact/detect before editing existing tracked symbols or committing. For pure directory moves, record that the expected blast radius is path-heavy and behavior-neutral.

## 2. Topology Documentation And Guard

- [x] 2.1 Add `macaca/crates/README.md` with the Route C layer model, old-to-new mapping, and rule that filesystem layer is not dependency permission.
- [x] 2.2 Add `route_c_workspace_topology` integration test module or file under `macaca-integration-tests`.
- [x] 2.3 Implement a package-to-layer topology registry for all current workspace crates.
- [x] 2.4 Use `cargo metadata --no-deps --format-version 1` in the topology guard to inspect package manifest paths.
- [x] 2.5 Fail unknown package names with actionable diagnostics requiring OpenSpec and topology map updates.
- [x] 2.6 Fail known packages located outside their expected `crates/<layer>/<crate>/Cargo.toml` suffix.
- [x] 2.7 Add detailed English comments in new Rust test/helper code explaining topology ownership, metadata traversal, diagnostics, and non-goals.

## 3. Cargo Workspace Path Update

- [x] 3.1 Update `macaca/Cargo.toml` workspace `members` paths to `crates/<layer>/<crate>`.
- [x] 3.2 Update `[workspace.dependencies]` internal crate `path` values to the same layer paths.
- [x] 3.3 Audit individual crate manifests for direct relative path dependencies and update only path values if needed.
- [x] 3.4 Run `cargo metadata --no-deps --format-version 1` and confirm all packages resolve from new paths.

## 4. Directory Moves

- [x] 4.1 Move `crates/macaca-proto` to `crates/foundation/macaca-proto`.
- [x] 4.2 Move `crates/macaca-ipc` to `crates/foundation/macaca-ipc`.
- [x] 4.3 Move `crates/macaca-persist` to `crates/foundation/macaca-persist`.
- [x] 4.4 Move `crates/macaca-kernel` to `crates/kernel/macaca-kernel`.
- [x] 4.5 Move `crates/macaca-task` to `crates/services/macaca-task`.
- [x] 4.6 Move `crates/macaca-llm` to `crates/services/macaca-llm`.
- [x] 4.7 Move `crates/macaca-memory` to `crates/services/macaca-memory`.
- [x] 4.8 Move `crates/macaca-context` to `crates/services/macaca-context`.
- [x] 4.9 Move `crates/macaca-driver` to `crates/services/macaca-driver`.
- [x] 4.10 Move `crates/macaca-skill` to `crates/services/macaca-skill`.
- [x] 4.11 Move `crates/macaca-gateway` to `crates/services/macaca-gateway`.
- [x] 4.12 Move `crates/macaca-tools` to `crates/services/macaca-tools`.
- [x] 4.13 Move `crates/macaca-runtime` to `crates/runtime/macaca-runtime`.
- [x] 4.14 Move `crates/macaca-runtime-host` to `crates/runtime/macaca-runtime-host`.
- [x] 4.15 Move `crates/macaca-framework` to `crates/runtime/macaca-framework`.
- [x] 4.16 Move `crates/macaca-agent` to `crates/application/macaca-agent`.
- [x] 4.17 Move `crates/macaca-app` to `crates/application/macaca-app`.
- [x] 4.18 Move `crates/macaca-sdk` to `crates/facade/macaca-sdk`.
- [x] 4.19 Move `crates/macaca-web` to `crates/shells/macaca-web`.
- [x] 4.20 Move `crates/macaca-cli` to `crates/shells/macaca-cli`.
- [x] 4.21 Move `crates/macaca-integration-tests` to `crates/tests/macaca-integration-tests`.
- [x] 4.22 Do not modify Rust source logic during the directory move slice except path-sensitive tests/helpers required by this proposal.

## 5. Path-Sensitive Script And Test Updates

- [x] 5.1 Update `route_c_baseline.rs` if it assumes integration tests live directly under `macaca/crates`.
- [x] 5.2 Update `route_c_dependency_boundaries/gate.rs` if it assumes integration tests live directly under `macaca/crates`.
- [x] 5.3 Update `task_api_migration_audit.rs` old flat paths to either new layer paths or metadata-derived package paths.
- [x] 5.4 Update `scripts/check-cli-consumer-migration.sh` to use layer-aware globs or metadata-derived paths.
- [x] 5.5 Update `scripts/check-web-cli-thin-shell.sh` to use layer-aware globs or metadata-derived paths.
- [x] 5.6 Audit `macaca/scripts/` and top-level `scripts/` for active `crates/macaca-*` path assumptions and update only executable scripts.
- [x] 5.7 Do not bulk rewrite historical research/OpenSpec prose unless it contains active command instructions that would fail after the move.

## 6. Governance Updates

- [x] 6.1 Update `macaca/docs/agent-os-microkernel-boundaries.md` current crate ownership table with the new layer paths.
- [x] 6.2 Update `macaca/docs/route-c-architecture-governance.md` to describe the workspace topology guard and its relationship to dependency boundary gate.
- [x] 6.3 Update `macaca/docs/route-c-serviceization-allowlist.md` only for topology clarification; do not remove allowlist rows merely because directories moved.
- [x] 6.4 Document that future new crates must be placed in a Route C layer and added to topology guard through OpenSpec.

## 7. Verification

- [x] 7.1 Run `openspec validate refactor-route-c-workspace-topology --strict`.
- [x] 7.2 Run `cargo metadata --no-deps --format-version 1`.
- [x] 7.3 Run `cargo fmt --all --check`.
- [x] 7.4 Run `cargo check --workspace`.
- [x] 7.5 Run `cargo test -p macaca-integration-tests route_c_workspace_topology`.
- [x] 7.6 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
- [x] 7.7 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 7.8 If path-sensitive package issues appear, run `cargo test -p macaca-sdk`, `cargo test -p macaca-runtime-host`, `cargo test -p macaca-web`, and `cargo test -p macaca-cli`.
- [x] 7.9 If scripts changed, run `scripts/check-cli-consumer-migration.sh` and `scripts/check-web-cli-thin-shell.sh`.
- [x] 7.10 Run `npx gitnexus detect-changes -r agent` before commit and review the affected scope.
- [x] 7.11 Run `npx gitnexus analyze` after directory moves or whenever GitNexus reports stale index.
