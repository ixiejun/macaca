## 1. Spec And Baseline

- [x] 1.1 Create `refactor-macaca-driver-patterns` proposal / design / tasks / delta spec.
- [x] 1.2 Review current `macaca-driver` implementation and upper-layer driver consumers.
- [x] 1.3 Run GitNexus impact for edited symbols and record risk.
- [x] 1.4 Run `openspec validate refactor-macaca-driver-patterns --strict`.

## 2. Slice 1: DriverFactory

- [x] 2.1 Add `DriverCreateContext`, `DriverFactory`, and `DynamicDriverFactory`.
- [x] 2.2 Migrate `DriverLoader::load_driver` to use `DynamicDriverFactory`.
- [x] 2.3 Mark `DynamicDriver::load` as deprecated with replacement guidance while keeping behavior.
- [x] 2.4 Add focused tests for dynamic factory context behavior where feasible.

## 3. Slice 2: DriverCommand

- [x] 3.1 Add `DriverCommand` for execute and execute-streaming actions.
- [x] 3.2 Route `DynamicTool::execute` through `DriverCommand`.
- [x] 3.3 Route `DynamicTool::execute_streaming` through `DriverCommand`.
- [x] 3.4 Keep non-streaming fallback behavior unchanged.

## 4. Slice 3: DriverTraceAdapter

- [x] 4.1 Add `DriverTraceAdapter` for driver trace enrichment.
- [x] 4.2 Replace inline streaming callback trace mutation with the adapter.
- [x] 4.3 Add tests that preserve driver identity and timestamp enrichment semantics.

## 5. Slice 4: DynamicDriverProxy

- [x] 5.1 Add `DynamicDriverSymbols` and `DynamicDriverProxy`.
- [x] 5.2 Route dynamic tool definition lookup through the proxy.
- [x] 5.3 Route execute / execute streaming / health / shutdown / destroy helpers through the proxy.
- [x] 5.4 Preserve dynamic library drop-order safety.

## 6. Slice 5: DriverSessionState

- [x] 6.1 Add `DriverSessionState` for streaming callback state.
- [x] 6.2 Route streaming trampoline state access through `DriverSessionState`.
- [x] 6.3 Keep callback state scoped to the blocking FFI call.

## 7. Verification

- [x] 7.1 Run `cargo test -p macaca-driver -- --nocapture`.
- [x] 7.2 Run `cargo check -p macaca-driver`.
- [x] 7.3 Run workspace `cargo check`.
- [x] 7.4 Run deprecated-call containment grep for old driver direct entrypoints.
- [x] 7.5 Run `gitnexus_detect_changes(scope: "all")`.
- [x] 7.6 Update this checklist so completed tasks reflect actual implementation.
