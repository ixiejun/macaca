## Context

The previous `refactor-macaca-driver-patterns` change introduced internal driver primitives:

- `DriverFactory`
- `DriverCommand`
- `DriverTraceAdapter`
- `DynamicDriverProxy`
- `DriverSessionState`

However, upper-layer consumers still perform driver lifecycle orchestration directly:

- web startup creates `DriverLoader`, calls `load_all`, counts tools, and registers drivers
- `/api/drivers/reload` clears the registry, calls `load_all`, counts tools, and registers drivers
- `build_toolkit` reaches into `DriverRegistry` for driver tools
- driver inventory is assembled in web routes from registry internals

## Goals

- Move driver lifecycle orchestration behind a `macaca-driver` facade.
- Keep runtime behavior 1:1.
- Migrate all current web consumer paths away from deprecated driver APIs.
- Keep old APIs present for later external migration.

## Non-Goals

- No driver ABI changes.
- No new reload semantics.
- No app-specific or driver-specific logic.
- No broad `AppState` cleanup in this slice.

## Decision

Use `DriverRuntime` as a Facade over `DriverLoader` and `DriverRegistry`.

Use `DriverLoadCommand` to represent load intent:

- `LoadAll`
- `Reload`

Use `DriverLoadReport` and `DriverLoadEntry` to carry load results and tool counts back to upper layers. This keeps `macaca-web` from manually calling `SoftwareDriver::tools` on freshly loaded drivers.

Keep `DriverRegistry` as the underlying state holder for this slice. `AppState` will hold both:

- `driver_registry`
- `driver_runtime`

Both must share the same `Arc<DriverRegistry>` to avoid split state.

## Alternatives Considered

### Only replace deprecated calls

This is too shallow because direct deprecated calls are already mostly removed. It leaves driver lifecycle orchestration in `macaca-web`.

### Replace `driver_registry` entirely

This gives the cleanest boundary, but touches too much state and test setup at once. It is deferred to a later cleanup slice.

## Migration Plan

1. Add `DriverRuntime` and load report types in `macaca-driver`.
2. Export runtime types.
3. Migrate web startup loading to runtime facade.
4. Add `driver_runtime` to `AppState` while preserving `driver_registry`.
5. Migrate driver list and reload routes.
6. Migrate toolkit driver tool collection to runtime facade.
7. Mark `DriverLoader::load_all` deprecated and keep internal runtime access through a non-deprecated crate-visible helper.

## Verification

- `openspec validate migrate-driver-consumers-to-runtime-primitives --strict`
- `cargo test -p macaca-driver -- --nocapture`
- `cargo check -p macaca-driver -p macaca-web -p macaca-integration-tests`
- workspace `cargo check`
- deprecated-call containment grep
- `gitnexus_detect_changes(scope: "all")`
