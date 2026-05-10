## 1. Preparation

- [x] 1.1 Read `docs/superpowers/plans/2026-05-10-s10-payment-a2a-serviceization-plan.md`, `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-serviceization-allowlist.md`, `macaca/docs/route-c-architecture-governance.md`, and `macaca/docs/design_patterns.md`.
- [x] 1.2 Review `add-a2a-payment-v0` implementation and confirm S10 builds on it instead of redefining basic A2A value objects.
- [x] 1.3 Run GitNexus impact before editing existing symbols: `A2ACoordinator`, `A2AProtocolAdapter`, `A2APaymentFacade`, `PaymentPolicyEngine`, `PaymentStore`, `SystemFacade`, and Web startup symbols touched by service registration.
- [x] 1.4 Warn before editing any HIGH or CRITICAL impact symbol.
- [x] 1.5 Confirm every touched Rust file remains under 500 lines; split DTO, admission, provider, adapter, and client logic before adding large code.

## 2. Payment Service Proto DTOs

- [x] 2.1 Add `macaca/crates/macaca-proto/src/payment_service.rs` with `PAYMENT_SERVICE_ID` and stable command names for quote, create intent, evaluate policy, approve intent, settle intent, receipt get/list, transition list, proof list, and snapshot.
- [x] 2.2 Define typed command/result DTOs using existing `a2a.rs` value objects: quote, intent create, policy evaluate, approve, settle, receipt get/list, transition list, proof list, snapshot.
- [x] 2.3 Define redacted views for lifecycle events, policy decisions, unavailable state, and service snapshots.
- [x] 2.4 Add English comments explaining provider-neutral boundaries, command semantics, state lifecycle, and redaction rules.
- [x] 2.5 Export `payment_service` from `macaca/crates/macaca-proto/src/lib.rs`.
- [x] 2.6 Add serde and command-name tests.
- [x] 2.7 Run `cargo test -p macaca-proto payment_service`.

## 3. Runtime-Host Admission Specifications

- [x] 3.1 Add `macaca/crates/macaca-runtime-host/src/payment_admission.rs`.
- [x] 3.2 Implement `PaymentTraceSpec`, `PaymentScopeSpec`, `PaymentAmountSpec`, `PaymentTransitionSpec`, and `PaymentRedactionSpec`.
- [x] 3.3 Reject mutating commands without valid `TraceContext` before adapter execution.
- [x] 3.4 Reject quote/intent commands without requester, provider, and capability scope.
- [x] 3.5 Reject invalid amount or invalid lifecycle transition with structured service errors.
- [x] 3.6 Add tests for trace rejection, scope rejection, redaction-key rejection, and invalid transition rejection.
- [x] 3.7 Run `cargo test -p macaca-runtime-host payment_admission`.

## 4. Runtime-Host Payment Service Provider

- [x] 4.1 Add `macaca/crates/macaca-runtime-host/src/payment_service_provider.rs`.
- [x] 4.2 Define `PaymentAdapterStrategy` and `LocalSimulatedPaymentAdapter` in runtime-host, reusing existing A2A contract semantics without depending on the kernel coordinator.
- [x] 4.3 Implement `PaymentSystemServiceProvider` as a `SystemService` that composes adapter, `PaymentPolicyEngine`, `PaymentStore`, admission specs, and structured logs.
- [x] 4.4 Implement quote command: validate trace/scope, call adapter quote, persist quote snapshot, return `QuoteResponse`.
- [x] 4.5 Implement create-intent command: create canonical intent, append initial transition, return `PaymentIntent`.
- [x] 4.6 Implement policy-evaluate command: validate amount/scope, evaluate budget/approval policy, return redacted decision view.
- [x] 4.7 Implement approve command: validate transition to approved and append transition.
- [x] 4.8 Implement settle command: validate policy/transition, execute local simulated adapter, persist proof and receipt, append settlement and receipt-recorded transitions.
- [x] 4.9 Implement receipt, transition, proof, and snapshot read commands.
- [x] 4.10 Emit structured logs for provider start, stop, call accepted, policy evaluated, transition appended, settlement completed, receipt recorded, and failure nodes.
- [x] 4.11 Ensure logs never include private keys, wallet secrets, provider credentials, raw signed payloads, API keys, raw provider responses, prompt bodies, raw package bytes, encrypted payload, or unbounded user input.
- [x] 4.12 Export provider and adapter types from `macaca/crates/macaca-runtime-host/src/lib.rs`.
- [x] 4.13 Run `cargo test -p macaca-runtime-host payment_service_provider`.

## 5. SDK Payment Client

- [x] 5.1 Add `macaca/crates/macaca-sdk/src/payment_client.rs`.
- [x] 5.2 Define `SystemPaymentClient` with quote, create intent, evaluate policy, approve, settle, receipt query, receipt list, transition list, proof list, and snapshot methods.
- [x] 5.3 Implement `ServiceBackedPaymentClient` over `SystemServiceClient` and typed `ServiceCallCommand`.
- [x] 5.4 Implement `UnavailableSystemPaymentClient` that fails closed for mutating/payment-required commands and returns structured unavailable snapshots for read-only calls.
- [x] 5.5 Add `payment_client` export from `macaca/crates/macaca-sdk/src/lib.rs`.
- [x] 5.6 Add `SystemFacade::payment_client()` and constructor/default wiring without constructing runtime-host providers.
- [x] 5.7 Add SDK tests for service-backed dispatch and unavailable fail-closed behavior.
- [x] 5.8 Run `cargo test -p macaca-sdk payment_client`.

## 6. Kernel Compatibility Migration

- [x] 6.1 Mark `A2ACoordinator` as deprecated with a note pointing to `PaymentSystemServiceProvider` and `SystemPaymentClient`.
- [x] 6.2 Mark `A2AProtocolAdapter`, `LocalSimulatedA2AAdapter`, and `A2APaymentFacade` as deprecated once runtime-host replacements exist.
- [x] 6.3 Keep `PaymentPolicyEngine` available as a kernel policy primitive unless implementation discovers it must move behind a dedicated service-policy crate.
- [x] 6.4 Preserve existing kernel A2A Payment v0 tests.
- [x] 6.5 Run `cargo test -p macaca-kernel a2a_payment` and `cargo test -p macaca-kernel payment_policy`.

## 7. Web Composition Root

- [x] 7.1 Register and start the built-in local simulated Payment Service in `macaca/crates/macaca-web/src/lib.rs` using the existing `ServiceRuntime` startup pattern.
- [x] 7.2 Wire `SystemPaymentClient` into Web state or `SystemFacade` only as a client/facade, not as payment semantics.
- [x] 7.3 Do not add Web payment business logic, marketplace UI, chain handling, provider special cases, or app-specific routes in S10.
- [x] 7.4 Run `cargo check -p macaca-web`.

## 8. Governance

- [x] 8.1 Add a Payment / A2A Service Ownership section to `macaca/docs/route-c-architecture-governance.md`.
- [x] 8.2 Update `macaca/docs/route-c-serviceization-allowlist.md` with S10 migration status and remaining debt.
- [x] 8.3 Update executable dependency boundary allowlist only if new direct dependency edges are introduced.
- [x] 8.4 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.

## 9. Verification

- [x] 9.1 Run `openspec validate add-payment-a2a-service-v1 --strict`.
- [x] 9.2 Run `cargo fmt --all --check`.
- [x] 9.3 Run `cargo test -p macaca-proto payment_service`.
- [x] 9.4 Run `cargo test -p macaca-persist payment_store`.
- [x] 9.5 Run `cargo test -p macaca-kernel a2a_payment` and `cargo test -p macaca-kernel payment_policy`.
- [x] 9.6 Run `cargo test -p macaca-runtime-host payment_admission` and `cargo test -p macaca-runtime-host payment_service_provider`.
- [x] 9.7 Run `cargo test -p macaca-sdk payment_client`.
- [x] 9.8 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
- [x] 9.9 Run `cargo check --workspace`.
- [x] 9.10 Run hardcode scan over new S10 code for app/workflow/provider/driver/gateway/model/chain/business-specific names.
- [x] 9.11 Run GitNexus `detect_changes` before commit and review affected scope.

