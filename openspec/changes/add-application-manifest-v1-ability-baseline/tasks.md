## 1. Model Contracts
- [x] 1.1 Add `ApplicationManifestV1`, manifest version, runtime profile, compatibility, commerce, plugin dependency, and UI declarations.
- [x] 1.2 Add `ApplicationAbilityDescriptor` and minimum ability kinds: Agent, UI, Headless, Scheduled, Gateway, Extension.
- [x] 1.3 Add permission, service requirement, capability, activation, and lifecycle policy declarations.
- [x] 1.4 Add deterministic serialization, sorting, and deduplication helpers where needed.

## 2. Application Framework Admission
- [x] 2.1 Add manifest and ability Specification modules in `macaca-app`.
- [x] 2.2 Add sanitized validation/admission report types.
- [x] 2.3 Export new modules without changing legacy YAML behavior.

## 3. Tests
- [x] 3.1 Add proto serialization/deserialization tests for Manifest v1.
- [x] 3.2 Add ability descriptor tests for every minimum ability kind.
- [x] 3.3 Add admission tests for missing trace, missing permission, missing service, and unsupported runtime declarations.
- [ ] 3.4 Run `cargo test -p macaca-proto application_manifest`.
- [ ] 3.5 Run `cargo test -p macaca-app manifest_v1`.
- [ ] 3.6 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
