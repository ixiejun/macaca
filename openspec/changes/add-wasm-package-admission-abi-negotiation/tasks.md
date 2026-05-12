## 1. Boundary Review
- [x] 1.1 Review Application Manifest v1, ability descriptors, WASM descriptor, compatibility checker, certification kit, SDK TestKit, and WIT schema.
- [x] 1.2 Run GitNexus impact analysis for planned checker/adapter symbols.

## 2. OpenSpec
- [x] 2.1 Add WASM package admission spec.
- [x] 2.2 Add WASM ABI negotiation spec.
- [x] 2.3 Add WASM compatibility/admission report spec.
- [x] 2.4 Validate OpenSpec change strictly.

## 3. Proto DTOs
- [x] 3.1 Add `WasmComponentArtifactDescriptor`, `WasmArtifactDigest`, and `WasmAbiRequirement`.
- [x] 3.2 Add `WasmImportRequirement`, `WasmExportDeclaration`, and `WasmAbiNegotiationResult`.
- [x] 3.3 Add stable sorting, serialization, and sanitization tests.

## 4. Application Admission
- [x] 4.1 Add artifact reference, ABI compatibility, required import permission, runtime capability, and report sanitization specifications.
- [x] 4.2 Add sanitized `WasmPackageAdmissionReport` projection.
- [x] 4.3 Preserve metadata-only WASM skeleton semantics through a legacy adapter path.
- [x] 4.4 Log admission decisions with trace id and stable reason codes.

## 5. SDK TestKit
- [x] 5.1 Add SDK TestKit validation for WASM artifact and ABI negotiation inputs.
- [x] 5.2 Ensure SDK diagnostics do not expose raw artifacts, raw manifests, or raw payloads.

## 6. Validation
- [x] 6.1 Run `cargo test -p macaca-proto wasm_abi`.
- [x] 6.2 Run `cargo test -p macaca-app wasm_admission`.
- [x] 6.3 Run `cargo test -p macaca-sdk application_testkit`.
- [x] 6.4 Run `openspec validate add-wasm-package-admission-abi-negotiation --strict`.
- [x] 6.5 Run GitNexus detect changes and verify affected scope.
