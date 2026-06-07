## 1. CertificationKit
- [x] 1.1 Add certification module and report DTOs.
- [x] 1.2 Add Visitor traversal over manifest, abilities, services, permissions, plugins, commerce, UI, and ABI.
- [x] 1.3 Add Specification rules for trace, permission, service, plugin dependency, runtime availability, commerce, UI, and redaction.

## 2. Fixtures
- [x] 2.1 Add declarative YAML/AgentAbility fixture.
- [x] 2.2 Add GenUI app fixture.
- [x] 2.3 Add headless app fixture.
- [x] 2.4 Add Store-entitled app fixture.
- [x] 2.5 Add Plugin-enhanced app fixture.
- [x] 2.6 Add WASM skeleton app fixture.

## 3. Integration Tests
- [x] 3.1 Add `application_platform_contracts.rs`.
- [x] 3.2 Test missing permission/service/plugin/runtime fail-closed or structured unavailable.
- [x] 3.3 Test sanitized metadata and certification reports do not leak unsafe payloads.
- [x] 3.4 Run `cargo test -p macaca-integration-tests --test application_platform_contracts`.
- [x] 3.5 Run `cargo test -p macaca-app certification`.
- [x] 3.6 Run `cargo test -p macaca-sdk application_testkit`.
- [x] 3.7 Run `cargo check --workspace`.
- [x] 3.8 Run `npx gitnexus detect-changes -r agent`.
