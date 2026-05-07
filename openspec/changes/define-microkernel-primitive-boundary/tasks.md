## 1. Preparation

- [x] 1.1 Re-read `docs/superpowers/plans/route-c-microkernel-ecosystem/phase-01-microkernel-primitive-boundary.md`.
- [x] 1.2 Re-read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-regression-matrix.md`, `macaca/docs/route-c-phase-template.md`, and `macaca/docs/route-c-architecture-governance.md`.
- [x] 1.3 Run GitNexus impact before editing existing symbols in `macaca-proto`, `macaca-kernel`, or `macaca-sdk`.
- [x] 1.4 Warn before proceeding if GitNexus reports HIGH or CRITICAL upstream risk.

## 2. `macaca-proto` primitives

- [x] 2.1 Add `macaca/crates/macaca-proto/src/kernel.rs`.
- [x] 2.2 Define `KernelServiceId`, `CapabilityId`, `CapabilityDescriptor`, `ServiceScope`, `TraceContext`, `PolicyRequest`, `PolicyDecision`, `ResourceScope`, and `KernelPrimitiveError`.
- [x] 2.3 Export the new module from `macaca/crates/macaca-proto/src/lib.rs`.
- [x] 2.4 Add serde round-trip tests for representative primitive values.
- [x] 2.5 Ensure all new code has detailed English comments explaining purpose and operating model.

## 3. `macaca-kernel` facade and registries

- [x] 3.1 Add `macaca/crates/macaca-kernel/src/facade.rs` with the additive `KernelFacade` trait and default facade implementation.
- [x] 3.2 Add `macaca/crates/macaca-kernel/src/capability_registry.rs` with `CapabilityRegistry` and in-memory descriptor registration/query.
- [x] 3.3 Add `macaca/crates/macaca-kernel/src/service_registry.rs` with `SystemServiceRegistry` and in-memory service registration/query.
- [x] 3.4 Add `macaca/crates/macaca-kernel/src/policy.rs` with `PolicyEngine`, default allow strategy, and test deny strategy.
- [x] 3.5 Add `macaca/crates/macaca-kernel/src/resource.rs` with `ResourceManager`, resource scope registration/query, and duplicate registration errors.
- [x] 3.6 Add a trace outlet trait for primitive events without binding to SSE, EventLog, or `macaca-web`.
- [x] 3.7 Export new additive modules from `macaca/crates/macaca-kernel/src/lib.rs`.
- [x] 3.8 Ensure all new code has detailed English comments explaining why the primitive belongs in kernel and which invariant it protects.

## 4. `macaca-sdk` additive access

- [x] 4.1 Add an SDK-facing facade access path in `macaca/crates/macaca-sdk/src/lib.rs`.
- [x] 4.2 Ensure SDK access does not depend on `macaca-web` or provider-specific crates.
- [x] 4.3 Document the facade as the preferred future path for applications and tooling to discover OS capabilities.

## 5. Deprecation guidance

- [x] 5.1 Identify direct kernel internals that have a safe tested facade alternative.
- [x] 5.2 Mark only those direct paths as `#[deprecated]` with a clear replacement message.
- [x] 5.3 Do not mark paths deprecated if no facade replacement exists in this phase.

Implementation note: Phase 01 introduced new additive primitive contracts and did not find an existing direct runtime path with a complete replacement facade in this slice, so no new deprecation attributes were added.

## 6. Verification

- [x] 6.1 Run `openspec validate define-microkernel-primitive-boundary --strict`.
- [x] 6.2 Run `cargo test -p macaca-proto`.
- [x] 6.3 Run `cargo test -p macaca-kernel`.
- [x] 6.4 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 6.5 Run `cargo check -p macaca-web`.
- [x] 6.6 Run `rg -n "FULLSTACK|NEWSROOM|claude|opencode|discord|telegram" macaca/crates/macaca-kernel/src macaca/crates/macaca-proto/src` and verify new kernel/proto primitive code does not introduce application, provider, driver, or gateway hardcode.
- [x] 6.7 Run `git diff --check`.
- [x] 6.8 Run GitNexus `detect_changes` before finalizing implementation.
