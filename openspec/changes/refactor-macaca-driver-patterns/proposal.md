# Change: Refactor macaca-driver toward factory, command, trace, proxy, and session primitives

## Why

`macaca-driver` is the Agent OS extension boundary for external software drivers. The current implementation works, but driver discovery, dynamic ABI loading, command execution, streaming trace conversion, and per-execution session state are still coupled inside a few broad modules, especially `loader.rs` and `dynamic_driver.rs`.

This change introduces design-pattern-based primitives in five small additive slices so the driver layer can keep existing behavior while becoming easier to extend, test, and migrate.

## What Changes

- Add a `DriverFactory` abstraction and dynamic factory implementation so `DriverLoader` no longer directly owns dynamic construction details.
- Add a `DriverCommand` abstraction so driver tool execution can be represented as a typed command instead of ad hoc `(tool_name, input, event_tx)` argument bundles.
- Add a `DriverTraceAdapter` abstraction so driver trace enrichment is centralized and keeps driver identity/timestamp behavior consistent.
- Add a `DynamicDriverProxy` abstraction to contain dynamic ABI calls used by `DynamicDriver` and dynamic tools.
- Add a `DriverSessionState` abstraction for streaming execution callback/session state so callback payloads are explicit and testable.
- Mark old direct construction/helper entrypoints as deprecated where a canonical replacement now exists, but do not delete them.

## Impact

- Affected specs:
  - `macaca-driver-core`
- Affected code:
  - `macaca/crates/macaca-driver/src/lib.rs`
  - `macaca/crates/macaca-driver/src/driver.rs`
  - `macaca/crates/macaca-driver/src/loader.rs`
  - `macaca/crates/macaca-driver/src/dynamic_driver.rs`
  - `macaca/crates/macaca-driver/src/registry.rs`
  - new additive modules under `macaca/crates/macaca-driver/src/`

## Risk

- Dynamic ABI risk is contained by keeping the existing C-ABI symbols and `DynamicDriver` field drop order intact.
- Trace behavior risk is contained by preserving current `driver_id` and timestamp enrichment semantics in the new adapter.
- Session/callback safety risk is contained by keeping callback state scoped to the blocking FFI call and not introducing detached callback lifetimes.
- Compatibility risk is contained by retaining old APIs with `deprecated` guidance rather than deleting them.

## Non-Goals

- Do not change the `SoftwareDriver` trait shape in this proposal.
- Do not change `plugin_abi` or bump `DRIVER_ABI_VERSION`.
- Do not remove `DynamicDriver::load`, `DriverLoader::load_driver`, `DriverRegistry::aggregate_tools`, or `DriverToolSet`.
- Do not hardcode driver names, application names, workflows, or business semantics.
- Do not migrate all upper-layer driver consumers in this proposal; this proposal only introduces canonical primitives and migrates internal `macaca-driver` implementation paths.
