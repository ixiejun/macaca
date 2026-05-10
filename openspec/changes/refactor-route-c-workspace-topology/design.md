# Design: Route C Workspace Topology

## Context

Route C has already established a logical architecture: a small microkernel, foundation contracts and service bus, replaceable system services, runtime-host service lifecycle, application framework, SDK facade, and thin presentation shells. Current Rust package names mostly reflect those responsibilities, but the directory layout still flattens every crate under `macaca/crates/`.

This proposal makes the architecture visible in the filesystem without changing package identity or runtime behavior. The topology move is intentionally mechanical: path structure changes, crate semantics do not.

## Goals

- Make Route C ownership boundaries visible from the directory tree.
- Preserve all existing package names, Rust crate names, public APIs, and behavior.
- Keep all Rust workspace packages under `macaca/crates/` to reduce path shock.
- Add an executable topology guard so new crates cannot bypass layer classification.
- Update active scripts/tests to avoid brittle flat-path assumptions.
- Keep dependency boundary gate authoritative for dependency permission.
- Keep the change reversible and auditable.

## Non-Goals

- No service behavior change.
- No new crate creation.
- No provider migration.
- No public API migration.
- No removal of deprecated compatibility paths.
- No historical OpenSpec rewrite in bulk.
- No attempt to split Store, Payment, Web3, EVM, UI, or Persistence into new crates.

## Target Topology

```text
macaca/crates/
  README.md
  foundation/
    macaca-proto/
    macaca-ipc/
    macaca-persist/
  kernel/
    macaca-kernel/
  services/
    macaca-task/
    macaca-llm/
    macaca-memory/
    macaca-context/
    macaca-driver/
    macaca-skill/
    macaca-gateway/
    macaca-tools/
  runtime/
    macaca-runtime/
    macaca-runtime-host/
    macaca-framework/
  application/
    macaca-agent/
    macaca-app/
  facade/
    macaca-sdk/
  shells/
    macaca-web/
    macaca-cli/
  tests/
    macaca-integration-tests/
```

## Layer Map

| Layer | Crates | Rationale |
| --- | --- | --- |
| `foundation` | `macaca-proto`, `macaca-ipc`, `macaca-persist` | Shared protocol/ABI/service DTOs, service bus/transport bridge, and persistence contract/current adapter foundation. |
| `kernel` | `macaca-kernel` | Microkernel invariants: registry, scheduler, policy, trace/task/session primitive ownership. |
| `services` | `macaca-task`, `macaca-llm`, `macaca-memory`, `macaca-context`, `macaca-driver`, `macaca-skill`, `macaca-gateway`, `macaca-tools` | Replaceable system service domains and provider-neutral capability surfaces. |
| `runtime` | `macaca-runtime`, `macaca-runtime-host`, `macaca-framework` | Agentic runtime primitives, host-owned service lifecycle, provider wrappers, traced framework/middleware execution seams. |
| `application` | `macaca-agent`, `macaca-app` | Agent primitives and Application Framework/package/lifecycle ownership. |
| `facade` | `macaca-sdk` | Shell/developer-facing SystemFacade and focused clients. |
| `shells` | `macaca-web`, `macaca-cli` | HTTP/Web/GenUI/trace viewer and terminal adapters. |
| `tests` | `macaca-integration-tests` | Cross-layer governance and regression tests. |

## Design Patterns

### Layers

Layer directories are the primary design pattern. The filesystem communicates ownership before a developer reads detailed docs.

### Facade

`crates/facade/macaca-sdk` remains the intended upper-layer facade. Moving it into `facade/` makes it visually distinct from service implementations and shells.

### Bridge

`crates/foundation/macaca-ipc` and `crates/runtime/macaca-runtime-host` represent bridge points between stable contracts, local service runtime, and future remote/plugin transports.

### Adapter

`crates/shells/*` and adapter-oriented service domains such as gateway/driver/skill remain visibly outside kernel/foundation. They adapt transports/providers/capabilities into system contracts.

### Registry

The topology guard owns an explicit package-to-layer table. New crates must be registered in that table before they can silently enter the workspace.

### Specification

The topology guard is an executable specification over `cargo metadata` package manifest paths. It complements, but does not replace, the existing dependency boundary specifications.

### Memento

`macaca/crates/README.md` records old-to-new path mapping so migration searches, docs, and rollback remain straightforward.

## Cargo Strategy

Cargo package names and Rust crate names stay unchanged. Only workspace member paths and `[workspace.dependencies]` path values change:

```toml
members = [
    "crates/foundation/macaca-proto",
    "crates/kernel/macaca-kernel",
    "crates/services/macaca-llm",
    ...
]

[workspace.dependencies]
macaca-proto = { path = "crates/foundation/macaca-proto" }
macaca-kernel = { path = "crates/kernel/macaca-kernel" }
```

Individual crate manifests should not need path edits if they already consume internal crates through workspace dependencies. If any crate uses direct relative path dependencies, update only those path values and do not change dependency semantics.

## Topology Guard Strategy

Add an integration test that:

1. Runs or consumes `cargo metadata --no-deps --format-version 1`.
2. Builds a map of workspace package name to manifest path.
3. Compares each current package to the expected `crates/<layer>/<crate>/Cargo.toml` suffix.
4. Fails unknown package names with an actionable diagnostic instructing maintainers to update OpenSpec and the topology map.
5. Fails known packages in the wrong layer.

The guard should include detailed English comments because it is executable architecture governance. Diagnostics should be deterministic and avoid relying on absolute machine-specific prefixes.

## Dependency Gate Interaction

The existing Route C dependency boundary gate remains authoritative for dependency permission. A crate being under `services/` does not automatically permit other crates to depend on it. The topology guard only answers “is this crate located in the right architectural folder?”; the dependency boundary gate answers “is this dependency edge allowed?”.

Do not delete allowlist rows in this change unless `cargo metadata` and dependency gate prove the direct dependency edge is gone for reasons unrelated to the directory move.

## Path Update Strategy

Update active paths in:

- `macaca/Cargo.toml`
- integration tests that compute workspace roots or inspect source paths,
- active scripts under `scripts/` and `macaca/scripts/`,
- current governance docs.

Do not bulk update historical research/OpenSpec prose. Old paths in historical documents remain valid as historical references. `macaca/crates/README.md` becomes the current topology source of truth.

## Logging And Trace

This is build/test topology infrastructure, not runtime execution. Runtime trace emission is not required. Auditability is provided by:

- OpenSpec proposal/design/tasks/spec,
- topology README,
- deterministic topology guard diagnostics,
- `cargo metadata` output,
- GitNexus detect/analyze after directory moves.

## Risks And Mitigations

- Risk: path-hardcoded scripts break.
  - Mitigation: audit active scripts and use `cargo metadata` or layer-aware globs where possible.
- Risk: directory move hides behavior changes in a huge diff.
  - Mitigation: keep Rust source logic unchanged; separate any behavior fix into another proposal.
- Risk: GitNexus index becomes stale.
  - Mitigation: run `npx gitnexus analyze` after the move or when the tool reports stale paths.
- Risk: rust-analyzer/editor caches old paths.
  - Mitigation: rely on Cargo metadata validation and workspace reload after the move.
- Risk: topology appears to grant dependency permission.
  - Mitigation: document that dependency gate remains authoritative and keep allowlist rows unchanged unless edges disappear.

## Verification Plan

- `openspec validate refactor-route-c-workspace-topology --strict`
- `cargo metadata --no-deps --format-version 1`
- `cargo fmt --all --check`
- `cargo check --workspace`
- `cargo test -p macaca-integration-tests route_c_workspace_topology`
- `cargo test -p macaca-integration-tests route_c_dependency_boundaries`
- `cargo test -p macaca-integration-tests --test route_c_baseline`
- Targeted tests if path-sensitive issues appear:
  - `cargo test -p macaca-sdk`
  - `cargo test -p macaca-runtime-host`
  - `cargo test -p macaca-web`
  - `cargo test -p macaca-cli`
- Updated scripts if touched:
  - `scripts/check-cli-consumer-migration.sh`
  - `scripts/check-web-cli-thin-shell.sh`
- GitNexus:
  - `npx gitnexus detect-changes -r agent`
  - `npx gitnexus analyze` after the move or when stale index is reported
