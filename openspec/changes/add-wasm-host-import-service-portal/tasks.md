## 1. Boundary and Impact
- [x] 1.1 Review `ApplicationHostCommand`, `ApplicationImport`, `ServiceRuntime`, WASM provider dispatch, SDK command builders, and service runtime tests.
- [x] 1.2 Run GitNexus impact analysis for existing host command and service runtime symbols.
- [x] 1.3 Confirm the bridge owns dispatch plumbing only and does not import concrete service business implementations.

## 2. OpenSpec
- [x] 2.1 Add host import categories and command schema spec.
- [x] 2.2 Add ServiceRuntime portal routing spec.
- [x] 2.3 Add sanitized host import audit spec.
- [x] 2.4 Add host import error taxonomy spec.
- [x] 2.5 Validate OpenSpec change strictly.

## 3. TDD Host Import Contract
- [x] 3.1 Add failing runtime-host tests for allowed service call, denied missing trace, missing capability, unavailable service, and sanitized output.
- [x] 3.2 Add failing SDK guest import contract test for service-call command shape.

## 4. Host Import DTOs and Bridge
- [x] 4.1 Add provider-neutral host import command/result/audit DTOs with detailed English comments.
- [x] 4.2 Add bridge validator for trace, payload bounds, capability metadata, and target service metadata.
- [x] 4.3 Add `WasmHostImportBridge` that routes service calls through `ServiceRuntime::call`.
- [x] 4.4 Map ServiceRuntime errors to structured application host command results.
- [x] 4.5 Sanitize and bound bridge output/result metadata.

## 5. Runtime Integration
- [x] 5.1 Add optional host import bridge to the default in-process WASM provider.
- [x] 5.2 Dispatch non-invoke host imports through the bridge while preserving export invocation behavior.
- [x] 5.3 Emit trace/log/audit metadata for requested, allowed, denied, unavailable, completed, and failed imports.

## 6. Validation
- [x] 6.1 Run `cargo test -p macaca-runtime-host wasm_host_import --manifest-path macaca/Cargo.toml`.
- [x] 6.2 Run `cargo test -p macaca-sdk wasm_guest_import_contract --manifest-path macaca/Cargo.toml`.
- [x] 6.3 Run `cargo test -p macaca-integration-tests --test application_platform_contracts --manifest-path macaca/Cargo.toml`.
- [x] 6.4 Run `openspec validate add-wasm-host-import-service-portal --strict`.
- [x] 6.5 Run GitNexus detect changes and verify affected scope.
