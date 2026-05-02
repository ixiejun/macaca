# Change: Migrate driver consumers to runtime primitives

## Why

`macaca-web` still manually orchestrates driver loading, reloading, inventory listing, and tool aggregation. This keeps driver lifecycle knowledge in the web entry layer instead of the driver infrastructure crate, even after `macaca-driver` gained factory, command, trace, proxy, and session primitives.

## What Changes

- Add a driver runtime facade in `macaca-driver`.
- Move startup load and reload orchestration behind that facade.
- Move driver inventory and tool aggregation consumer paths to that facade.
- Keep legacy registry/loader APIs as deprecated compatibility wrappers.
- Preserve existing `/api/drivers/reload` JSON shape and startup auto-load behavior.

## Impact

- Affected specs:
  - `macaca-driver-core`
- Affected code:
  - `macaca/crates/macaca-driver/src/lib.rs`
  - `macaca/crates/macaca-driver/src/loader.rs`
  - `macaca/crates/macaca-driver/src/registry.rs`
  - new `macaca-driver` runtime/load command modules
  - `macaca/crates/macaca-web/src/lib.rs`
  - `macaca/crates/macaca-web/src/state.rs`
  - `macaca/crates/macaca-web/src/routes.rs`
  - `macaca/crates/macaca-web/src/framework_toolkit.rs`

## Risk

- `AppState`, `start_server`, and `build_toolkit` have high blast radius because they participate in server startup and agent construction. This migration must keep the old `driver_registry` field and share the same registry instance with the new runtime facade.
- Reload behavior must stay clear-then-load. This proposal does not add rollback-on-failure semantics.
- Tool count behavior must stay equivalent to the existing `SoftwareDriver::tools(driver.as_ref()).len()` call.

## Non-Goals

- Do not change `SoftwareDriver`.
- Do not change dynamic driver ABI.
- Do not remove `DriverLoader`, `DriverRegistry`, or compatibility wrappers.
- Do not change the driver reload API response shape.
- Do not add driver health monitoring, source providers, permission policy, or driver-specific logic.
