## 1. Spec And Baseline

- [x] 1.1 Create proposal, design, tasks, and delta spec.
- [x] 1.2 Review current driver consumer paths.
- [x] 1.3 Run GitNexus impact for edited symbols.
- [x] 1.4 Validate OpenSpec.

## 2. Driver Runtime Facade

- [x] 2.1 Add `DriverLoadCommand`, `DriverLoadStatus`, `DriverLoadEntry`, and `DriverLoadReport`.
- [x] 2.2 Add `DriverRuntime` and `DriverInventoryItem`.
- [x] 2.3 Export runtime facade types from `macaca-driver`.
- [x] 2.4 Add focused runtime tests.

## 3. Web Startup Migration

- [x] 3.1 Add `driver_runtime` to `AppState` while preserving `driver_registry`.
- [x] 3.2 Create a shared `Arc<DriverRegistry>` and `Arc<DriverRuntime>` at startup.
- [x] 3.3 Replace startup load loop with `DriverRuntime::load_all`.
- [x] 3.4 Preserve existing startup logging behavior.

## 4. Routes And Toolkit Migration

- [x] 4.1 Migrate `get_drivers` to `DriverRuntime::list_inventory`.
- [x] 4.2 Migrate `reload_drivers` to `DriverRuntime::reload`.
- [x] 4.3 Migrate `build_toolkit` to `DriverRuntime::collect_tools`.
- [x] 4.4 Preserve existing reload response JSON shape.

## 5. Deprecated Call Containment

- [x] 5.1 Mark `DriverLoader::load_all` deprecated and route runtime through a crate-visible internal helper.
- [x] 5.2 Verify web no longer calls deprecated driver loading or registry aggregation APIs.
- [x] 5.3 Keep old APIs present for external migration.

## 6. Verification

- [x] 6.1 Run `openspec validate migrate-driver-consumers-to-runtime-primitives --strict`.
- [x] 6.2 Run `cargo test -p macaca-driver -- --nocapture`.
- [x] 6.3 Run `cargo check -p macaca-driver -p macaca-web -p macaca-integration-tests`.
- [x] 6.4 Run workspace `cargo check`.
- [x] 6.5 Run deprecated-call containment grep.
- [x] 6.6 Run `gitnexus_detect_changes(scope: "all")`.
- [x] 6.7 Update this checklist to match actual completion.
