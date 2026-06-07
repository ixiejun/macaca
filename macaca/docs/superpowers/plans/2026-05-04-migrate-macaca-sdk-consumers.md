# Migrate macaca-sdk Consumers Brainstorm and Plan

Date: 2026-05-04

## 1. Current Code Facts

This plan follows:

- `macaca/docs/design-pattern-refactor-plans/README.md`
- `macaca/docs/design-pattern-refactor-plans/refactor-order.md`
- `macaca/docs/design-pattern-refactor-plans/macaca-sdk.md`
- `macaca/docs/design_patterns.md`
- `openspec/AGENTS.md`

`macaca-sdk` has already completed the producer-side design-pattern refactor in `openspec/changes/refactor-macaca-sdk-patterns`:

- `macaca/crates/macaca-sdk/src/spec.rs` defines `AgentSpec`, `AgentSpecBuilder`, and `TracePolicy`.
- `macaca/crates/macaca-sdk/src/facade.rs` defines `MacacaSdk`, `AgentRegistryApi`, and `KernelAgentRegistry`.
- `macaca/crates/macaca-sdk/src/persona_prototype.rs` defines persona prototype cloning primitives.
- `macaca/crates/macaca-sdk/src/validation.rs` defines `SdkValidationChain`.
- `AgentBuilder::build`, `AgentBuilder::build_with_manifest`, `register_from_config`, and `register_from_file` remain present but are marked deprecated and delegate through the new primitives.

Remaining upper consumers found by source scan:

- `macaca/crates/macaca-app/src/runtime.rs`
  - Production path still imports and calls deprecated `macaca_sdk::register_from_config`.
  - This is the highest priority migration because app startup uses it to register declarative agents.
- `macaca/crates/macaca-integration-tests/tests/kernel_lifecycle.rs`
  - Tests still call deprecated `AgentBuilder::build_with_manifest`.
  - One test still verifies deprecated `macaca_sdk::register_from_config`.
- `macaca/crates/macaca-integration-tests/tests/live_fullstack_autodev.rs`
  - Ignored live test still calls deprecated `AgentBuilder::build_with_manifest`.
- `macaca/crates/macaca-kernel/tests/e2e_auto_programming.rs`
  - Kernel tests still call deprecated `AgentBuilder::build_with_manifest`.
- `macaca/crates/macaca-app/src/loader.rs`
  - Consumes `AgentConfig` and `CapabilityDef`; this remains valid because app manifest resolution should produce SDK configs before SDK facade registration.
- `macaca/crates/macaca-web/src/framework_runner.rs`
  - Consumes `AgentPersona`; no deprecated SDK registration/build API usage was found in production code.

OpenSpec context:

- `openspec list` shows `refactor-macaca-sdk-patterns` is complete.
- `openspec list --specs` currently reports no baseline specs, so the consumer migration should add a delta under a new change, not modify a baseline spec.

## 2. Superpowers Brainstorm

### Option A: Migrate only production app runtime

Replace `macaca-app` startup registration from deprecated `register_from_config` to `MacacaSdk::for_kernel(kernel).register_config(config)`.

Benefits:

- Smallest production behavior change.
- Removes the main runtime usage of the deprecated SDK helper.
- Low chance of changing kernel registration semantics because the facade currently delegates to the same kernel path.

Risks:

- Tests and ignored live flows still call deprecated APIs.
- The repository remains noisy under `rg "build_with_manifest|register_from_config"`.
- Future migration work may miss test-only deprecated usage that still documents old patterns.

### Option B: Migrate production path plus tests to facade/spec

Use `MacacaSdk::for_kernel(...).register_config(...)` where the caller wants "register config into kernel", and use `AgentBuilder::build_spec()` plus `AgentSpec::manifest()` / `AgentSpec::into_agent()` where the test specifically needs manual kernel registration.

Benefits:

- Removes deprecated SDK API usage from all upper consumer code outside `macaca-sdk`'s own compatibility tests.
- Keeps test intent explicit: facade for registration behavior, spec for manual kernel registration behavior.
- Preserves current runtime semantics while aligning consumers with Builder + Facade + Adapter boundaries.

Risks:

- Touches multiple test files and one production file.
- Manual conversions with `AgentSpec::manifest()` and `into_agent()` must keep the current manifest-before-agent order, because `into_agent()` consumes the spec.
- Kernel tests may continue to test SDK-built declarative agents, so failures could look like kernel regressions even if caused by SDK migration.

### Option C: Introduce a new app-owned SDK registration adapter

Add an `ApplicationAgentRegistrar` in `macaca-app` that wraps `MacacaSdk`, then migrate app runtime through that app-owned facade.

Benefits:

- Gives `macaca-app` a narrow seam for future trace policy or app-level registration metadata.
- Keeps `AppRuntime` less aware of SDK facade details.

Risks:

- Adds another abstraction immediately after `MacacaSdk` already introduced the facade.
- Current production usage has only one registration loop, so this is likely over-designed for this migration.
- Could blur responsibility between app runtime assembly and SDK registration.

### Option D: Keep deprecated APIs in tests intentionally

Migrate only production code and keep tests that assert deprecated compatibility.

Benefits:

- Maintains explicit coverage that deprecated APIs still work.
- Lowest churn.

Risks:

- Conflicts with the user goal that upper code still calling deprecated APIs should be migrated.
- Makes deprecated APIs look like recommended usage in kernel/integration tests.
- Leaves later search-based migration harder.

## 3. Recommendation

Choose Option B.

Rationale:

- It fully addresses the migration goal without changing SDK or kernel runtime semantics.
- It uses the design patterns already introduced by the SDK refactor:
  - Builder: `AgentBuilder::build_spec()` or `AgentSpec::from_config(...)`.
  - Facade: `MacacaSdk::for_kernel(...).register_config(...)`.
  - Adapter: `KernelAgentRegistry` remains hidden behind the facade.
- It avoids adding a new app-level abstraction until there is repeated app-specific registration behavior.
- It keeps deprecated APIs in `macaca-sdk` only, where compatibility tests may remain with explicit `#[allow(deprecated)]` if needed.

## 4. Migration Targets

Production code:

- `macaca/crates/macaca-app/src/runtime.rs`
  - Replace `register_from_config(kernel, config).await?` with a `MacacaSdk::for_kernel(kernel)` facade.
  - Create the facade once before the registration loop.

Tests:

- `macaca/crates/macaca-integration-tests/tests/kernel_lifecycle.rs`
  - Replace direct `build_with_manifest()` manual registration with `build_spec()`, `spec.manifest()`, and `spec.into_agent()`.
  - Replace deprecated `register_from_config` test with `MacacaSdk::for_kernel(...).register_config(...)`.
  - Rename the deprecated helper test so it describes facade registration.
- `macaca/crates/macaca-integration-tests/tests/live_fullstack_autodev.rs`
  - Replace `build_with_manifest()` with `build_spec()` after prompt override.
- `macaca/crates/macaca-kernel/tests/e2e_auto_programming.rs`
  - Replace all `build_with_manifest()` usage with a local helper that returns `(DeclarativeAgent, AgentManifest)` through `build_spec()`.
  - Keep these tests focused on kernel execution behavior, not deprecated SDK compatibility.

Optional follow-up:

- `macaca-sdk` internal compatibility tests may keep deprecated APIs with local `#[allow(deprecated)]` because the old APIs intentionally remain available for migration discovery.

## 5. Risks and Controls

- Risk: `macaca-app` startup is production-critical.
  Control: Use `MacacaSdk::for_kernel(kernel).register_config(config)`, which currently delegates to the same kernel registration path as `register_from_config`.

- Risk: `AgentSpec::into_agent()` consumes the spec.
  Control: Always derive `manifest` before calling `into_agent()`, or use a small local helper in tests.

- Risk: Test migration may hide compatibility coverage.
  Control: Leave deprecated compatibility coverage inside `macaca-sdk` itself, not in upper consumer crates.

- Risk: Trace policy is metadata-only today.
  Control: Do not claim traced runtime construction is complete in this migration; only ensure upper consumers no longer use deprecated SDK entry points.

- Risk: High GitNexus impact due to app startup and kernel/integration tests.
  Control: Before implementation, run impact analysis for `register_from_config`, `AgentBuilder::build_with_manifest`, `MacacaSdk::register_config`, and `AgentBuilder::build_spec`; report HIGH/CRITICAL results before edits.

## 6. Write Plan

### Task 1: OpenSpec Change

Create `openspec/changes/migrate-sdk-consumers-to-facade-spec/`:

- `proposal.md`
  - Explain that SDK producer refactor is complete and upper consumers must move to facade/spec primitives.
- `design.md`
  - Document the facade/spec migration strategy and why no new app-level registrar is introduced.
- `tasks.md`
  - Track production migration, test migration, deprecated scan, validation, and checklist updates.
- `specs/macaca-sdk-consumers/spec.md`
  - Add requirements that upper consumers use `MacacaSdk` for registration and `AgentSpec` for SDK-built manual registration.

Run:

```bash
openspec validate migrate-sdk-consumers-to-facade-spec --strict
```

### Task 2: Production Migration

Modify `macaca/crates/macaca-app/src/runtime.rs`:

- Import `macaca_sdk::MacacaSdk` instead of deprecated `register_from_config`.
- Instantiate `let sdk = MacacaSdk::for_kernel(kernel);` before the agent registration loop.
- Register each config through `sdk.register_config(config).await?`.

Validation:

```bash
cargo check -p macaca-app
```

### Task 3: Integration Test Migration

Modify `macaca/crates/macaca-integration-tests/tests/kernel_lifecycle.rs`:

- Add a small helper or inline pattern:
  - `let spec = AgentBuilder::from_config(config).build_spec().unwrap();`
  - `let agent_id = spec.id();`
  - `let manifest = spec.manifest();`
  - `let agent = spec.into_agent();`
- Replace facade registration test with `MacacaSdk::for_kernel(&kernel).register_config(config).await`.

Modify `macaca/crates/macaca-integration-tests/tests/live_fullstack_autodev.rs`:

- Replace `build_with_manifest()` with `build_spec()` plus manifest/agent conversion.

Validation:

```bash
cargo test -p macaca-integration-tests kernel_lifecycle -- --nocapture
```

### Task 4: Kernel Test Migration

Modify `macaca/crates/macaca-kernel/tests/e2e_auto_programming.rs`:

- Add a local helper such as `build_agent_from_config(config)` that returns `(DeclarativeAgent, AgentManifest)` through `build_spec()`.
- Replace all `build_with_manifest()` call sites with that helper.

Validation:

```bash
cargo test -p macaca-kernel --test e2e_auto_programming -- --nocapture
```

### Task 5: Deprecated Usage Scan

Run:

```bash
rg -n "register_from_config|register_from_file|build_with_manifest|AgentBuilder::from_config\\([^\\n]*\\)\\.build\\(" macaca/crates --glob '*.rs'
```

Expected result:

- No upper consumer usage of deprecated SDK entry points.
- Remaining matches are allowed only inside `macaca-sdk` compatibility implementation/tests, with local `#[allow(deprecated)]` where compile warnings require it.

### Task 6: Full Validation

Run focused and cross-crate checks:

```bash
cargo test -p macaca-sdk -- --nocapture
cargo check -p macaca-sdk -p macaca-app -p macaca-kernel -p macaca-integration-tests
openspec validate migrate-sdk-consumers-to-facade-spec --strict
```

Before commit or final handoff:

```bash
npx gitnexus detect-changes --repo agent --scope all
```

If GitNexus reports stale index:

```bash
npx gitnexus analyze
```

## 7. Definition of Done

- `macaca-app` production startup no longer calls deprecated `macaca_sdk::register_from_config`.
- Upper consumer tests no longer call `AgentBuilder::build_with_manifest` or deprecated registry helpers.
- Deprecated SDK APIs remain in `macaca-sdk` for migration discovery and compatibility.
- OpenSpec proposal/design/tasks/spec are valid and aligned.
- Focused tests and cross-crate checks pass or failures are documented with exact commands and causes.
