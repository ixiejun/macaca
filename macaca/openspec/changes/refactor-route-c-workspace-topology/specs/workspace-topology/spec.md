## ADDED Requirements

### Requirement: Macaca SHALL organize Rust workspace crates by Route C architecture layer

Macaca SHALL place Rust workspace crates under `macaca/crates/<layer>/<crate>/` directories that reflect Route C ownership layers instead of keeping all crates as flat peers.

#### Scenario: Workspace exposes Route C layer directories

- **WHEN** a maintainer lists `macaca/crates/`
- **THEN** the workspace SHALL include layer directories for `foundation`, `kernel`, `services`, `runtime`, `application`, `facade`, `shells`, and `tests`
- **AND** each current Rust workspace crate SHALL live under exactly one of those layer directories

#### Scenario: Existing crates map to expected layers

- **WHEN** the topology refactor is complete
- **THEN** `macaca-proto`, `macaca-ipc`, and `macaca-persist` SHALL live under `crates/foundation/`
- **AND** `macaca-kernel` SHALL live under `crates/kernel/`
- **AND** `macaca-task`, `macaca-llm`, `macaca-memory`, `macaca-context`, `macaca-driver`, `macaca-skill`, `macaca-gateway`, and `macaca-tools` SHALL live under `crates/services/`
- **AND** `macaca-runtime`, `macaca-runtime-host`, and `macaca-framework` SHALL live under `crates/runtime/`
- **AND** `macaca-agent` and `macaca-app` SHALL live under `crates/application/`
- **AND** `macaca-sdk` SHALL live under `crates/facade/`
- **AND** `macaca-web` and `macaca-cli` SHALL live under `crates/shells/`
- **AND** `macaca-integration-tests` SHALL live under `crates/tests/`

### Requirement: Macaca SHALL preserve crate identity and runtime behavior during topology refactor

The topology refactor SHALL move directories without changing Rust package names, crate names, public APIs, service contracts, command names, route paths, CLI commands, wire formats, deprecated compatibility anchors, or runtime behavior.

#### Scenario: Cargo package identity remains stable

- **WHEN** `cargo metadata --no-deps --format-version 1` is run after the move
- **THEN** it SHALL list the same 21 workspace package names as before the move
- **AND** no package SHALL be renamed as part of this change

#### Scenario: User-visible behavior remains unchanged

- **WHEN** the topology refactor is implemented
- **THEN** `/api/chat/v2`, SSE trace, session replay, task board, application lifecycle, Web UI, CLI commands, Store/Entitlement, Payment/A2A, Web3/EVM unavailable behavior, and existing provider compatibility paths SHALL preserve their behavior
- **AND** no deprecated compatibility anchor SHALL be deleted by this topology-only change

### Requirement: Macaca SHALL validate workspace topology through executable metadata checks

Macaca SHALL provide an executable topology guard that uses Cargo workspace metadata to verify crate layer placement.

#### Scenario: Topology guard validates package manifest paths

- **WHEN** the topology guard runs
- **THEN** it SHALL execute or consume `cargo metadata --no-deps --format-version 1`
- **AND** it SHALL compare each workspace package manifest path against the expected `crates/<layer>/<crate>/Cargo.toml` suffix
- **AND** it SHALL fail deterministically if a package is missing, unknown, or located in the wrong layer

#### Scenario: New crate requires topology classification

- **WHEN** a new Rust workspace crate is added
- **THEN** the topology guard SHALL fail until the crate is assigned to a Route C layer through OpenSpec and the topology registry is updated
- **AND** the diagnostic SHALL tell maintainers to update the topology map and architecture docs

### Requirement: Macaca SHALL keep dependency permissions separate from filesystem layer placement

Filesystem layer placement SHALL NOT grant dependency permission. The Route C dependency boundary gate and allowlist SHALL remain authoritative for forbidden dependency edges.

#### Scenario: Service layer placement does not bypass dependency gate

- **GIVEN** a crate lives under `crates/services/`
- **WHEN** another crate adds a direct dependency on it
- **THEN** the existing Route C dependency boundary gate SHALL still evaluate whether that dependency edge is allowed
- **AND** topology placement alone SHALL NOT allow kernel, presentation shell, optional module, or provider reverse dependencies

#### Scenario: Allowlist rows are not removed by path-only movement

- **WHEN** crates are moved into Route C layer directories
- **THEN** existing serviceization allowlist rows SHALL remain until `cargo metadata` and the dependency boundary gate prove the underlying direct dependency edge is removed
- **AND** directory movement alone SHALL NOT be treated as migration completion

### Requirement: Macaca SHALL document the workspace topology and old-to-new path mapping

Macaca SHALL document the new Route C workspace topology so maintainers can understand layer ownership, update scripts, and find moved crates.

#### Scenario: Crates README explains topology

- **WHEN** a maintainer opens `macaca/crates/README.md`
- **THEN** it SHALL describe each Route C layer, list the crates in each layer, and provide an old-to-new path mapping
- **AND** it SHALL state that historical OpenSpec/research paths may still reference old flat paths
- **AND** it SHALL state that new executable scripts and tests should prefer `cargo metadata` or layer-aware globs over hardcoded flat paths

### Requirement: Macaca SHALL update active path-sensitive tooling for the new topology

Active tests and scripts that must run after the topology refactor SHALL be updated to the new layer paths or changed to discover package paths from metadata.

#### Scenario: Path-sensitive checks continue to run

- **WHEN** topology refactor validation runs
- **THEN** active route C integration tests and shell migration scripts SHALL use valid paths for the new layer layout
- **AND** they SHALL NOT assume all crates live directly under `macaca/crates/macaca-*`
- **AND** historical prose-only documents SHALL NOT need bulk rewrites unless they contain active command instructions
