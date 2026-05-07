## 1. Preparation

- [x] 1.1 Read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-regression-matrix.md`, `macaca/docs/route-c-phase-template.md`, `macaca/docs/route-c-architecture-governance.md`, and `docs/superpowers/plans/route-c-microkernel-ecosystem/phase-04-package-manifest-runtime-guard.md`.
- [x] 1.2 Review existing manifest and loader code in `macaca-proto`, `macaca-app`, `macaca-skill`, `macaca-driver`, and `macaca-runtime-host`.
- [x] 1.3 Run GitNexus impact before modifying each selected symbol; warn before editing any HIGH or CRITICAL impact symbol.

## 2. Protocol Contracts

- [x] 2.1 Add `macaca-proto/src/package.rs` with Package Manifest v0 value objects and data contracts.
- [x] 2.2 Export package contracts from `macaca-proto/src/lib.rs`.
- [x] 2.3 Add serde tests covering all required package types and runtime kinds.
- [x] 2.4 Add tests proving unknown future package type/runtime kind is represented as structured data and does not panic.

## 3. YAML Application Compatibility Adapter

- [x] 3.1 Add `macaca-app/src/package.rs` with a package descriptor builder and YAML `AppManifest` compatibility adapter.
- [x] 3.2 Preserve app id, name, version, entry agent, entrypoint/workflow references, agent capabilities, allowed tools, and required/optional service metadata where available.
- [x] 3.3 Add tests using at least two real YAML application fixtures or repository sample applications to prove current apps can produce package descriptors without hardcoded app names.

## 4. Runtime Guard Chain

- [x] 4.1 Add `macaca-app/src/runtime_guard.rs` with guard traits and ordered guard steps using Chain of Responsibility and Specification patterns.
- [x] 4.2 Reject missing runtime kind with a structured error.
- [x] 4.3 Reject incompatible ABI or OS version with a structured error.
- [x] 4.4 Reject missing required services with a structured error.
- [x] 4.5 Mark missing optional services unavailable without rejecting the package.
- [x] 4.6 Parse and preserve commerce metadata without enforcing payment or entitlement.
- [x] 4.7 Emit structured logs and presentation-neutral trace/audit events for guard step start, pass, rejection, optional-service degradation, and final decision.

## 5. Package Loader Factory

- [x] 5.1 Add `macaca-app/src/package_loader.rs` with runtime-kind based loader selection.
- [x] 5.2 Implement YAML package metadata loading through the existing `AppLoader` compatibility path.
- [x] 5.3 Add a WASM component metadata loader stub that returns structured `RuntimeUnavailable` for execution when no WASM runtime is installed.
- [x] 5.4 Add tests proving unsupported package/runtime combinations fail with explainable structured errors.

## 6. Skill / Driver / Runtime Host Descriptor Hooks

- [x] 6.1 Add additive descriptor conversion hooks for skill metadata if the existing skill model can map safely without changing runtime behavior.
- [x] 6.2 Add additive descriptor conversion hooks for driver manifests if the existing driver model can map safely without changing runtime behavior.
- [x] 6.3 Add runtime-host package requirement or compatibility hook only if it remains additive and does not force plugin/MCP runtime migration.
- [x] 6.4 Add targeted tests for every hook added in this slice.

## 7. Regression And Verification

- [x] 7.1 Run `openspec validate add-package-manifest-runtime-guard --strict`.
- [x] 7.2 Run `cargo test -p macaca-proto package`.
- [x] 7.3 Run `cargo test -p macaca-app package_manifest`.
- [x] 7.4 Run targeted tests for skill, driver, and runtime-host hooks if they are added.
- [x] 7.5 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 7.6 Run `cargo check -p macaca-web`.
- [x] 7.7 Run `cargo check --workspace`.
- [x] 7.8 Run a hardcode scan over new package files for demo app names, workflow names, provider names, driver names, gateway names, model names, chain names, and business-specific routing.
- [x] 7.9 Run `gitnexus_detect_changes(scope: "all")` before committing and verify affected flows match the expected Phase 04 scope.
