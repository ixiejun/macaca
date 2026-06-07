## 1. Preparation

- [x] 1.1 Read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-regression-matrix.md`, `macaca/docs/route-c-phase-template.md`, `macaca/docs/route-c-architecture-governance.md`, `docs/superpowers/plans/2026-05-07-macaca-os-route-c-microkernel-ecosystem-plan.md`, and `docs/superpowers/plans/route-c-microkernel-ecosystem/phase-13-ecosystem-hardening.md`.
- [x] 1.2 Review existing Phase 04-12 code and specs for package manifest, Application ABI, GenUI, plugin runtime, Store/Entitlement, A2A payment, optional Web3/EVM, and Web/CLI thin shell.
- [x] 1.3 Run GitNexus impact before modifying selected implementation symbols; warn before editing HIGH or CRITICAL impact symbols.

## 2. Developer Documentation

- [x] 2.1 Create `macaca/docs/developer/application-development-guide.md` covering YAML apps, WASM-stub apps, package manifest, permissions, trace, debugging, and certification.
- [x] 2.2 Create `macaca/docs/developer/plugin-development-guide.md` covering gateway plugins, driver plugins, lifecycle, capabilities, permissions, trace, and unavailable-safe behavior.
- [x] 2.3 Create `macaca/docs/developer/genui-development-guide.md` covering UI schema, UI events, trace context, renderer unavailable behavior, and certification.
- [x] 2.4 Create `macaca/docs/developer/store-submission-guide.md` covering package metadata, signature metadata, entitlement states, paid/free/open packages, encrypted metadata expectations, and certification.
- [x] 2.5 Create `macaca/docs/developer/web3-dapp-development-guide.md` covering optional Web3, optional EVM/DApp, unavailable-safe behavior, trace, permissions, and Store implications.
- [x] 2.6 Update `macaca/docs/SYSTEM_OVERVIEW.md` to link ecosystem hardening docs and explain that third-party packages must pass certification without modifying Macaca source.

## 3. SDK Package Fixtures

- [x] 3.1 Add `macaca/crates/macaca-sdk/examples/yaml_app_fixture.rs` producing a generic YAML application package descriptor.
- [x] 3.2 Add `macaca/crates/macaca-sdk/examples/wasm_stub_app_fixture.rs` producing a WASM metadata package descriptor that remains execution-unavailable.
- [x] 3.3 Add `macaca/crates/macaca-sdk/examples/genui_app_fixture.rs` producing a traced GenUI package fixture.
- [x] 3.4 Add `macaca/crates/macaca-sdk/examples/gateway_plugin_fixture.rs` producing a generic gateway plugin descriptor without naming a real provider.
- [x] 3.5 Add `macaca/crates/macaca-sdk/examples/driver_plugin_fixture.rs` producing a generic driver plugin descriptor without naming a real driver.
- [x] 3.6 Add `macaca/crates/macaca-sdk/examples/paid_skill_fixture.rs` producing paid and free skill package descriptors.
- [x] 3.7 Add `macaca/crates/macaca-sdk/examples/web3_optional_fixture.rs` producing an optional Web3 application descriptor.
- [x] 3.8 Add `macaca/crates/macaca-sdk/examples/evm_optional_fixture.rs` producing an optional EVM/DApp descriptor.

## 4. Compatibility Checker

- [x] 4.1 Add `macaca/crates/macaca-app/src/compatibility_checker.rs` with checker input, host context, report, diagnostic, trace event, severity, and status value objects.
- [x] 4.2 Implement checker rules with Specification pattern for manifest version, runtime kind, ABI version, package type, permissions, required services, optional modules, commerce metadata, trace metadata, and upgrade compatibility.
- [x] 4.3 Implement Visitor traversal over package descriptor sections so new package fields can be checked without rewriting a monolithic `if/else` function.
- [x] 4.4 Implement Facade entrypoint `PackageCompatibilityChecker` and export it from `macaca-app/src/lib.rs`.
- [x] 4.5 Emit structured `tracing` logs for checker start, each rule pass/warn/fail, optional module unavailable, entitlement diagnostics, and final status.
- [x] 4.6 Add unit tests for compatible package, warning package, incompatible package, missing required service, optional service unavailable, ABI warning, ABI rejection, paid package entitlement missing, and future package/runtime kinds.

## 5. Certification Tests

- [x] 5.1 Add `macaca/crates/macaca-integration-tests/tests/package_certification.rs` using a Template Method style certification harness.
- [x] 5.2 Certify YAML app fixture as compatible.
- [x] 5.3 Certify WASM-stub fixture as metadata-compatible and execution-unavailable.
- [x] 5.4 Certify GenUI fixture with trace context and schema validation.
- [x] 5.5 Certify gateway and driver plugin fixtures with capability/permission/lifecycle metadata.
- [x] 5.6 Certify paid skill fixture with entitlement deny and entitlement allow reports.
- [x] 5.7 Certify Web3 optional fixture and EVM optional fixture when optional modules are unavailable.
- [x] 5.8 Certify invalid fixtures return structured diagnostics and never panic, hang, or silently pass.

## 6. Upgrade Compatibility Policy

- [x] 6.1 Define compatibility rules for OS version, Application ABI version, package manifest version, and runtime kind.
- [x] 6.2 Ensure checker distinguishes `compatible`, `compatible_with_warnings`, and `incompatible`.
- [x] 6.3 Ensure downgrade/unknown-future-version diagnostics are structured and actionable.
- [x] 6.4 Document upgrade policy in developer docs and `SYSTEM_OVERVIEW.md`.

## 7. Regression And Verification

- [x] 7.1 Run `openspec validate add-ecosystem-hardening --strict`.
- [x] 7.2 Run `cargo test -p macaca-app compatibility_checker`.
- [x] 7.3 Run `cargo test -p macaca-integration-tests package_certification`.
- [x] 7.4 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 7.5 Run `cargo check -p macaca-web`.
- [x] 7.6 Run `cargo check --workspace`.
- [x] 7.7 Run `rg -n "YAML|WASM|GenUI|Plugin|Store|Web3|EVM" macaca/docs/developer`.
- [x] 7.8 Run hardcode scans over new docs, examples, checker, and tests for application names, provider names, driver names, gateway names, model names, chain names, and business-specific routing.
- [x] 7.9 Run `gitnexus_detect_changes(scope: "all")` before committing and verify affected scope matches Phase 13.
