## 1. ABI Schema
- [x] 1.1 Add `macaca-application.wit` or equivalent ABI schema.
- [x] 1.2 Align schema imports/exports with `ApplicationImport` and `ApplicationExport`.
- [x] 1.3 Add tests preventing ABI/schema drift.

## 2. SDK and Descriptor
- [x] 2.1 Add `WasmComponentApplicationDescriptor`.
- [x] 2.2 Add SDK scaffold helpers for WASM skeleton fixtures.
- [x] 2.3 Add documentation comments explaining guest/host boundary and unavailable behavior.

## 3. Host Skeleton
- [x] 3.1 Add `WasmApplicationHostFactory`.
- [x] 3.2 Add `UnavailableWasmApplicationHost`.
- [x] 3.3 Ensure host dispatch returns structured runtime-unavailable with trace and reason.

## 4. Validation
- [x] 4.1 Add WASM skeleton fixture tests.
- [x] 4.2 Run `cargo test -p macaca-proto application_abi`.
- [x] 4.3 Run `cargo test -p macaca-app wasm`.
- [x] 4.4 Run `cargo test -p macaca-runtime-host application_hosts`.
- [x] 4.5 Run `cargo check --workspace`.
