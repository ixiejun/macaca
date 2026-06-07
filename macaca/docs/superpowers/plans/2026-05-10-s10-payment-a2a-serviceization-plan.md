# S10 Payment / A2A 服务化 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 A2A quote、payment intent、budget/approval、settlement、receipt、execution proof 从 kernel helper 收敛为 Route C Payment Service，并保留微内核只拥有 policy primitive 的边界。

**Architecture:** 采用 additive-first：先新增 provider-neutral Payment Service contract、runtime-host provider、SDK client 和 Web composition root，再把旧 kernel coordinator 标注 deprecated。Payment lifecycle 通过 Command + State + Memento + Observer 表达，adapter/provider/payment rail 通过 Strategy / Adapter / Bridge 扩展。

**Tech Stack:** Rust workspace, `macaca-proto`, `macaca-kernel`, `macaca-persist`, `macaca-runtime-host`, `macaca-sdk`, `macaca-web`, ServiceRuntime, OpenSpec, GitNexus, Route C dependency gate.

---

## Scope

S10 覆盖：

- Payment Service provider-neutral DTO：quote、intent create、policy evaluate、approval、settlement、receipt query、transition/proof query、snapshot。
- Runtime-host Payment Service provider：decode command、validate admission、execute local simulated adapter、persist mementos、emit trace/log。
- SDK focused client：`SystemPaymentClient`、service-backed client、unavailable client、`SystemFacade` accessor。
- Web startup composition：注册并启动 built-in Payment Service，但 Web 不拥有 payment semantics。
- Kernel compatibility：旧 `A2ACoordinator` / adapter path 标注 deprecated，生产新路径优先通过 Payment Service。
- Governance docs：更新 Route C architecture governance 与 allowlist，明确 Payment/A2A ownership 和迁移债务。

S10 不覆盖：

- 真实支付 provider、银行卡、企业账单、链上支付、钱包签名、外部 payment gateway。
- S11 Web3/EVM optional module、链上交易、智能合约调用。
- Store/Entitlement 授权规则本身。
- Web payment approval UI / marketplace billing UI 的完整实现。
- 删除旧 kernel A2A coordinator。

## Required Governance Inputs

- `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`
- `docs/superpowers/plans/2026-05-10-s10-payment-a2a-serviceization-brainstorm.md`
- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/design_patterns.md`
- `macaca/docs/route-c-regression-matrix.md` if present in the checkout when implementation starts.

## Architecture Decision

Use a focused Payment Service, not a generic commerce macro-service:

- `Payment Service` owns quote, payment intent lifecycle, policy evaluation command surface, approval state, settlement adapter dispatch, receipt/proof persistence, and snapshot.
- `Kernel` keeps only policy primitive/facade, service registry, trace/audit primitive, and compatibility anchors.
- `Store/Entitlement Service` can later call Payment Service for paid package flows, but S10 does not move Store/Entitlement logic.
- `Web/CLI/Gateway` are shell adapters and must use SDK clients for any new payment surface.

Design patterns:

- Facade: `SystemPaymentClient` and `Payment Service` hide provider/runtime details.
- Mediator: runtime-host provider coordinates adapter, policy, store, and observer.
- Strategy: payment adapter, budget policy, approval policy, receipt/proof issuer.
- Command: service operations are typed commands before `ServiceCommand` dispatch.
- State: `PaymentIntentState` remains the canonical lifecycle guard.
- Memento: quote, transition, receipt, proof, snapshot are persisted/replayed artifacts.
- Observer: trace/audit/log events are emitted for every lifecycle node.
- Adapter / Bridge: local simulated adapter and future provider plugins sit behind provider-neutral contracts.
- Null Object: unavailable client/provider returns structured unavailable for payment-required commands.
- Specification: trace/scope/amount/transition/redaction rules are centralized.

Rejected alternatives:

- SDK-only wrapper around `macaca-kernel::A2ACoordinator`: rejected because it does not serviceize Payment/A2A.
- Big-bang removal of kernel coordinator: rejected because it is not additive-first and would remove migration search anchors.
- Payment logic inside Store/Entitlement: rejected because S9 and S10 are separate service boundaries.

## Proposed OpenSpec Change

Expected change id:

- `add-payment-a2a-service-v1`

Expected artifacts:

- `openspec/changes/add-payment-a2a-service-v1/proposal.md`
- `openspec/changes/add-payment-a2a-service-v1/design.md`
- `openspec/changes/add-payment-a2a-service-v1/tasks.md`
- `openspec/changes/add-payment-a2a-service-v1/specs/payment-service/spec.md`
- `openspec/changes/add-payment-a2a-service-v1/specs/payment-sdk-client/spec.md`
- `openspec/changes/add-payment-a2a-service-v1/specs/payment-consumer-migration/spec.md`
- `openspec/changes/add-payment-a2a-service-v1/specs/payment-audit-trace/spec.md`

The proposal must state:

- Payment/A2A belongs to Payment Service, not kernel, Web, CLI, Store, Entitlement, Web3, or EVM.
- Kernel policy primitive may remain, but adapter orchestration and payment lifecycle execution must move to runtime-host service provider.
- Every mutating payment command requires `TraceContext`.
- Payment-required paths fail closed when Payment Service is unavailable, disabled, denied, over budget, approval missing, adapter missing, or transition invalid.
- Read-only receipt/list/snapshot commands may return empty unavailable views, but must include diagnostics.
- Logs, trace events, receipts, proofs, and snapshots must not expose private keys, wallet secrets, provider credentials, raw signed payloads, API keys, raw remote provider responses, prompt bodies, raw package bytes, or encrypted payload.
- No application/provider/driver/gateway/model/chain/business-specific name can be hardcoded into control flow.

## Implementation Slices

### Slice S10.1: Impact And Boundary Audit

**Files:**

- Inspect: `macaca/crates/macaca-proto/src/a2a.rs`
- Inspect: `macaca/crates/macaca-kernel/src/a2a.rs`
- Inspect: `macaca/crates/macaca-kernel/src/a2a_event.rs`
- Inspect: `macaca/crates/macaca-kernel/src/payment_policy.rs`
- Inspect: `macaca/crates/macaca-persist/src/payment_store.rs`
- Inspect: `macaca/crates/macaca-runtime-host/src/service_runtime.rs`
- Inspect: `macaca/crates/macaca-runtime-host/src/service_provider.rs`
- Inspect: `macaca/crates/macaca-runtime-host/src/store_service_provider.rs`
- Inspect: `macaca/crates/macaca-sdk/src/service_client.rs`
- Inspect: `macaca/crates/macaca-sdk/src/system_facade.rs`
- Inspect: `macaca/crates/macaca-web/src/lib.rs`
- Inspect: `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries.rs`

- [ ] **Step 1: Read current code and OpenSpec instructions**

Run:

```bash
sed -n '1,220p' openspec/AGENTS.md
sed -n '1,260p' macaca/docs/agent-os-microkernel-boundaries.md
sed -n '1,260p' macaca/docs/route-c-serviceization-allowlist.md
sed -n '1,260p' macaca/docs/route-c-architecture-governance.md
```

Expected: docs confirm Payment/A2A belongs to Payment Service and optional payment adapters must not enter kernel.

- [ ] **Step 2: Run GitNexus impact before editing existing symbols**

Run impact for at least:

```text
impact upstream target=A2ACoordinator
impact upstream target=A2AProtocolAdapter
impact upstream target=PaymentPolicyEngine
impact upstream target=PaymentStore
impact upstream target=SystemFacade
```

Expected: report direct callers and risk. If HIGH/CRITICAL, stop and warn before editing.

- [ ] **Step 3: Classify current Payment/A2A paths**

Document in the OpenSpec design:

```text
macaca-proto/src/a2a.rs                    => protocol DTO baseline
macaca-kernel/src/payment_policy.rs        => kernel policy primitive
macaca-kernel/src/a2a.rs                   => deprecated compatibility coordinator
macaca-kernel/src/a2a_event.rs             => trace/audit observer primitive or compatibility observer
macaca-persist/src/payment_store.rs        => repository/memento adapter
macaca-runtime-host/src/payment_service_provider.rs => new runtime-host provider
macaca-sdk/src/payment_client.rs           => new SDK facade
macaca-web/src/lib.rs                      => composition root only
```

Expected: no new provider/app/workflow hardcode is planned.

### Slice S10.2: OpenSpec Proposal And Delta Specs

**Files:**

- Create: `openspec/changes/add-payment-a2a-service-v1/proposal.md`
- Create: `openspec/changes/add-payment-a2a-service-v1/design.md`
- Create: `openspec/changes/add-payment-a2a-service-v1/tasks.md`
- Create: `openspec/changes/add-payment-a2a-service-v1/specs/payment-service/spec.md`
- Create: `openspec/changes/add-payment-a2a-service-v1/specs/payment-sdk-client/spec.md`
- Create: `openspec/changes/add-payment-a2a-service-v1/specs/payment-consumer-migration/spec.md`
- Create: `openspec/changes/add-payment-a2a-service-v1/specs/payment-audit-trace/spec.md`

- [ ] **Step 1: Write proposal**

Proposal must include:

```markdown
# add-payment-a2a-service-v1

## Why

Payment/A2A is currently coordinated by kernel compatibility helpers. Route C requires Payment/A2A to become a replaceable system service while kernel keeps only policy primitives and service registry invariants.

## What Changes

- Add provider-neutral Payment Service DTOs.
- Add runtime-host Payment Service provider over policy/store/adapter strategies.
- Add SDK `SystemPaymentClient` and `SystemFacade` accessor.
- Register built-in local simulated Payment Service from the host composition root.
- Mark kernel A2A coordinator/adapter helpers deprecated.
- Update Route C governance and allowlist with S10 migration state.

## Non-Goals

- No real external payment provider.
- No chain, wallet, Web3, EVM, or smart contract integration.
- No marketplace billing UI.
- No deletion of existing compatibility APIs.
```

- [ ] **Step 2: Write design**

Design must explicitly cover:

```markdown
## Pattern Choices

- Facade: Payment Service and SystemPaymentClient.
- Mediator: runtime-host provider coordinates policy, adapter, store, observer.
- Strategy: payment adapter and payment policy.
- Command: all calls enter as typed commands.
- State: PaymentIntentState guards transitions.
- Memento: quote/transition/receipt/proof persistence.
- Observer: trace/audit event emission.
- Null Object: unavailable client/provider behavior.
- Specification: trace/scope/redaction/transition validation.

## Boundary Rules

Kernel must not own adapter execution. Web/CLI must not define payment semantics. Payment Service must not perform Store/Entitlement policy or Web3/EVM execution.
```

- [ ] **Step 3: Write delta specs**

Minimum requirements:

```markdown
### Requirement: Payment Service Commands
Payment Service SHALL expose provider-neutral quote, create-intent, evaluate-policy, approve, settle, receipt query, transition query, proof query, and snapshot commands.

#### Scenario: mutating payment command without trace
- WHEN a mutating payment command lacks TraceContext
- THEN the service SHALL reject it before adapter execution

### Requirement: Payment Service Audit
Payment Service SHALL emit bounded trace/audit records for every lifecycle transition and SHALL NOT include secrets or raw provider payloads.

### Requirement: SDK Payment Client
SDK SHALL expose SystemPaymentClient and unavailable/service-backed implementations.

### Requirement: Compatibility Migration
Existing kernel A2A coordinator APIs SHALL remain available but deprecated, and new production consumers SHALL use Payment Service clients.
```

- [ ] **Step 4: Validate OpenSpec**

Run:

```bash
openspec validate add-payment-a2a-service-v1 --strict
```

Expected: `Change 'add-payment-a2a-service-v1' is valid`.

### Slice S10.3: Payment Service DTOs In `macaca-proto`

**Files:**

- Add: `macaca/crates/macaca-proto/src/payment_service.rs`
- Modify: `macaca/crates/macaca-proto/src/lib.rs`

- [ ] **Step 1: Add command names and service id**

Define:

```rust
/// Provider-neutral service id used by ServiceRuntime and SDK clients.
pub const PAYMENT_SERVICE_ID: &str = "macaca.payment";

pub const PAYMENT_QUOTE_COMMAND: &str = "payment.quote";
pub const PAYMENT_INTENT_CREATE_COMMAND: &str = "payment.intent.create";
pub const PAYMENT_POLICY_EVALUATE_COMMAND: &str = "payment.intent.evaluate_policy";
pub const PAYMENT_INTENT_APPROVE_COMMAND: &str = "payment.intent.approve";
pub const PAYMENT_INTENT_SETTLE_COMMAND: &str = "payment.intent.settle";
pub const PAYMENT_RECEIPT_GET_COMMAND: &str = "payment.receipt.get";
pub const PAYMENT_RECEIPT_LIST_COMMAND: &str = "payment.receipt.list";
pub const PAYMENT_TRANSITION_LIST_COMMAND: &str = "payment.transition.list";
pub const PAYMENT_PROOF_LIST_COMMAND: &str = "payment.proof.list";
pub const PAYMENT_SNAPSHOT_COMMAND: &str = "payment.snapshot";
```

- [ ] **Step 2: Add typed commands/results**

Use existing `a2a.rs` types. Required DTOs:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentQuoteCommand {
    pub trace: TraceContext,
    pub request: QuoteRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentCreateCommand {
    pub trace: TraceContext,
    pub quote: QuoteResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentPolicyEvaluateCommand {
    pub trace: TraceContext,
    pub intent: PaymentIntent,
    pub budget: BudgetPolicy,
    pub approval: ApprovalPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentApproveCommand {
    pub trace: TraceContext,
    pub intent_id: PaymentIntentId,
    pub approval: ApprovalPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntentSettleCommand {
    pub trace: TraceContext,
    pub intent: PaymentIntent,
}
```

Also add receipt/transition/proof list commands and `PaymentServiceSnapshot`.

- [ ] **Step 3: Add redacted views**

Add redacted views instead of exposing raw adapter payload:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentLifecycleEventView {
    pub trace: TraceContext,
    pub operation: String,
    pub status: String,
    pub quote_id: Option<QuoteId>,
    pub intent_id: Option<PaymentIntentId>,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub reason: Option<String>,
    pub metadata: BTreeMap<String, String>,
}
```

English comments must explain that the view is bounded and must not contain secrets.

- [ ] **Step 4: Add tests**

Add tests in `macaca-proto/src/payment_service.rs` or a `payment_service_tests.rs` module:

```rust
#[test]
fn payment_service_command_names_are_stable() {
    assert_eq!(PAYMENT_SERVICE_ID, "macaca.payment");
    assert_eq!(PAYMENT_QUOTE_COMMAND, "payment.quote");
}
```

Run:

```bash
cargo test -p macaca-proto payment_service
```

Expected: tests pass.

### Slice S10.4: Payment Admission Specifications

**Files:**

- Add: `macaca/crates/macaca-runtime-host/src/payment_admission.rs`

- [ ] **Step 1: Implement trace and scope specs**

Add small Specification objects:

```rust
pub struct PaymentTraceSpec;
pub struct PaymentScopeSpec;
pub struct PaymentAmountSpec;
pub struct PaymentTransitionSpec;
pub struct PaymentRedactionSpec;
```

Responsibilities:

- `PaymentTraceSpec` rejects missing or empty trace id.
- `PaymentScopeSpec` checks requester, provider, and capability id are non-empty for quote/intent operations.
- `PaymentAmountSpec` checks amount quantity parses through existing `PaymentAmount::as_f64`.
- `PaymentTransitionSpec` delegates to `PaymentIntentState::can_transition_to`.
- `PaymentRedactionSpec` provides a helper for safe metadata keys and documents forbidden payloads.

- [ ] **Step 2: Add tests**

Tests:

```rust
#[test]
fn payment_scope_spec_rejects_empty_requester() {
    // Build a QuoteRequest with empty requester id and assert ServiceError.
}

#[test]
fn payment_redaction_spec_rejects_secret_keys() {
    // Check keys like "private_key", "api_key", "wallet_secret" are rejected.
}
```

Run:

```bash
cargo test -p macaca-runtime-host payment_admission
```

Expected: tests pass.

### Slice S10.5: Runtime-Host Payment Service Provider

**Files:**

- Add: `macaca/crates/macaca-runtime-host/src/payment_service_provider.rs`
- Modify: `macaca/crates/macaca-runtime-host/src/lib.rs`

- [ ] **Step 1: Define provider structure**

Provider dependencies:

```rust
pub struct PaymentSystemServiceProvider {
    descriptor: ServiceDescriptor,
    adapter: Arc<dyn PaymentAdapterStrategy>,
    policy: Arc<dyn PaymentPolicyEngine>,
    store: Arc<dyn PaymentStore>,
}
```

Add English comments explaining that provider owns orchestration, while adapter/policy/store remain replaceable strategies.

- [ ] **Step 2: Move local simulated adapter semantics behind runtime-host Strategy**

Add:

```rust
#[async_trait]
pub trait PaymentAdapterStrategy: Send + Sync {
    fn is_configured(&self) -> bool;
    async fn quote(&self, request: QuoteRequest) -> Result<QuoteResponse, A2AError>;
    async fn settle(&self, intent: &PaymentIntent) -> Result<(PaymentReceipt, ExecutionProof), A2AError>;
}
```

Implement `LocalSimulatedPaymentAdapter` using the current `LocalSimulatedA2AAdapter` semantics, but do not depend on kernel coordinator.

- [ ] **Step 3: Implement `SystemService`**

Provider `call` must match command names:

```rust
match command.name.as_str() {
    PAYMENT_QUOTE_COMMAND => { /* decode PaymentQuoteCommand, validate, quote, store quote */ }
    PAYMENT_INTENT_CREATE_COMMAND => { /* create intent transition */ }
    PAYMENT_POLICY_EVALUATE_COMMAND => { /* evaluate policy */ }
    PAYMENT_INTENT_APPROVE_COMMAND => { /* transition to approved */ }
    PAYMENT_INTENT_SETTLE_COMMAND => { /* transition executing->settled->receipt_recorded */ }
    PAYMENT_RECEIPT_GET_COMMAND => { /* query store */ }
    PAYMENT_RECEIPT_LIST_COMMAND => { /* query by session/task */ }
    PAYMENT_TRANSITION_LIST_COMMAND => { /* query transitions */ }
    PAYMENT_PROOF_LIST_COMMAND => { /* query proofs */ }
    PAYMENT_SNAPSHOT_COMMAND => { /* sanitized snapshot */ }
    _ => Err(ServiceError::UnsupportedCommand(command.name)),
}
```

Every branch logs service id, command, trace id, quote id/intent id if present, status, and reason code. Logs must not include secrets.

- [ ] **Step 4: Add provider tests**

Required tests:

```rust
#[tokio::test]
async fn payment_service_quote_persists_quote_snapshot() { ... }

#[tokio::test]
async fn payment_service_settle_records_receipt_and_proof() { ... }

#[tokio::test]
async fn payment_service_rejects_command_without_trace() { ... }

#[tokio::test]
async fn payment_service_rejects_invalid_transition() { ... }
```

Run:

```bash
cargo test -p macaca-runtime-host payment_service_provider
```

Expected: tests pass.

### Slice S10.6: SDK Payment Client And SystemFacade

**Files:**

- Add: `macaca/crates/macaca-sdk/src/payment_client.rs`
- Modify: `macaca/crates/macaca-sdk/src/system_facade.rs`
- Modify: `macaca/crates/macaca-sdk/src/lib.rs`

- [ ] **Step 1: Add focused client trait**

Define:

```rust
#[async_trait]
pub trait SystemPaymentClient: Send + Sync {
    async fn quote(&self, command: PaymentQuoteCommand) -> MacacaResult<QuoteResponse>;
    async fn create_intent(&self, command: PaymentIntentCreateCommand) -> MacacaResult<PaymentIntent>;
    async fn evaluate_policy(&self, command: PaymentPolicyEvaluateCommand) -> MacacaResult<PaymentPolicyDecisionView>;
    async fn approve(&self, command: PaymentIntentApproveCommand) -> MacacaResult<PaymentIntent>;
    async fn settle(&self, command: PaymentIntentSettleCommand) -> MacacaResult<PaymentReceipt>;
    async fn receipt(&self, command: PaymentReceiptGetCommand) -> MacacaResult<Option<PaymentReceipt>>;
    async fn snapshot(&self, command: PaymentSnapshotCommand) -> MacacaResult<PaymentServiceSnapshot>;
}
```

If `PaymentPolicyDecisionView` does not exist in proto, add it in Slice S10.3 as a redacted decision DTO.

- [ ] **Step 2: Add service-backed and unavailable clients**

Follow S9 pattern from `store_client.rs`:

- `ServiceBackedPaymentClient` serializes typed command into `ServiceCallCommand`.
- `UnavailableSystemPaymentClient` returns structured unavailable for mutating commands and empty unavailable snapshot for snapshot/read-only commands.

- [ ] **Step 3: Add SystemFacade accessor**

Modify `SystemFacade`:

```rust
pub fn payment_client(&self) -> Arc<dyn SystemPaymentClient> {
    Arc::clone(&self.payment)
}
```

Use constructor defaults that preserve existing tests.

- [ ] **Step 4: Add SDK tests**

Tests:

```rust
#[tokio::test]
async fn service_backed_payment_client_dispatches_quote_command() { ... }

#[tokio::test]
async fn unavailable_payment_client_fails_closed_for_settle() { ... }
```

Run:

```bash
cargo test -p macaca-sdk payment_client
```

Expected: tests pass.

### Slice S10.7: Kernel Compatibility Deprecation

**Files:**

- Modify: `macaca/crates/macaca-kernel/src/a2a.rs`
- Modify: `macaca/crates/macaca-kernel/src/a2a_event.rs` if needed
- Modify: `macaca/crates/macaca-kernel/src/payment_policy.rs` only if needed

- [ ] **Step 1: Mark compatibility coordinator and adapter as deprecated**

Add:

```rust
#[deprecated(note = "Use PaymentSystemServiceProvider plus SystemPaymentClient for new Payment/A2A call paths")]
pub struct A2ACoordinator { ... }
```

Also mark:

- `A2AProtocolAdapter`
- `LocalSimulatedA2AAdapter`
- `A2APaymentFacade`
- `local_simulated_terms` if moved to runtime-host replacement exists.

- [ ] **Step 2: Keep policy primitive non-deprecated unless moved**

`PaymentPolicyEngine` may remain as kernel policy primitive. Do not mark it deprecated unless OpenSpec explicitly moves policy to a separate service contract.

- [ ] **Step 3: Verify compatibility tests**

Run:

```bash
cargo test -p macaca-kernel a2a_payment
cargo test -p macaca-kernel payment_policy
```

Expected: tests still pass. Deprecation warnings are acceptable.

### Slice S10.8: Web Composition Root Registration

**Files:**

- Modify: `macaca/crates/macaca-web/src/lib.rs`
- Modify: `macaca/crates/macaca-web/src/state.rs` only if a stored `payment_client` is needed.

- [ ] **Step 1: Register built-in Payment Service at startup**

Use existing Web startup pattern from Store/Entitlement:

```rust
let payment_provider = macaca_runtime_host::PaymentSystemServiceProvider::local_simulated(
    Arc::new(macaca_persist::InMemoryPaymentStore::new()),
);
service_runtime.register(payment_provider).await?;
service_runtime.start(PAYMENT_SERVICE_ID).await?;
```

The exact constructor may differ after Slice S10.5; keep Web as composition root only.

- [ ] **Step 2: Add SDK client to AppState if needed**

If `AppState` already carries `SystemFacade`, prefer that. If not, add:

```rust
pub payment_client: Arc<dyn macaca_sdk::SystemPaymentClient>,
```

Do not add payment semantics to Web routes in this slice.

- [ ] **Step 3: Verify Web build**

Run:

```bash
cargo check -p macaca-web
```

Expected: check passes.

### Slice S10.9: Governance And Allowlist Updates

**Files:**

- Modify: `macaca/docs/route-c-architecture-governance.md`
- Modify: `macaca/docs/route-c-serviceization-allowlist.md`
- Modify: `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries.rs` only if a new allowlist row is required.

- [ ] **Step 1: Add Payment/A2A ownership section**

Add governance section after S9:

```markdown
## Payment / A2A Service Ownership

Payment Service owns quote, payment intent lifecycle, approval state, settlement adapter dispatch, receipt/proof persistence, and payment snapshots. Kernel owns only policy primitive and service registry invariants. Web/CLI/Gateway/Application must consume Payment Service through SDK clients.
```

- [ ] **Step 2: Update allowlist**

Document remaining debt:

```markdown
| Edge | Current S10 status | Remaining debt |
| --- | --- | --- |
| `macaca-kernel -> macaca-persist` | Legacy A2A coordinator still uses PaymentStore compatibility path. | Remove when all consumers use Payment Service and kernel coordinator is deleted. |
| `macaca-web -> macaca-runtime-host` | Web registers built-in Payment Service as composition root. | Move provider composition to shared runtime bootstrap in S12. |
```

Only add executable dependency gate allowlist rows if `cargo metadata` introduces new forbidden direct edges.

- [ ] **Step 3: Verify dependency boundaries**

Run:

```bash
cargo test -p macaca-integration-tests route_c_dependency_boundaries
```

Expected: passes. If it fails due new dependency, update docs and test allowlist through the OpenSpec.

### Slice S10.10: Final Verification

**Files:**

- Verify all touched files; split any Rust file above 500 LOC before completion.

- [ ] **Step 1: Run focused tests**

Run:

```bash
openspec validate add-payment-a2a-service-v1 --strict
cargo fmt --all --check
cargo test -p macaca-proto payment_service
cargo test -p macaca-persist payment_store
cargo test -p macaca-kernel a2a_payment
cargo test -p macaca-kernel payment_policy
cargo test -p macaca-runtime-host payment_admission
cargo test -p macaca-runtime-host payment_service_provider
cargo test -p macaca-sdk payment_client
cargo test -p macaca-integration-tests route_c_dependency_boundaries
```

Expected: all commands pass.

- [ ] **Step 2: Run workspace check**

Run:

```bash
cargo check --workspace
```

Expected: workspace check passes. Existing warnings may remain; new warnings should be reviewed.

- [ ] **Step 3: Run hardcode scan**

Run:

```bash
rg -n "FULLSTACK|NEWSROOM|claude|opencode|discord|telegram|ethereum|stripe|paypal|openai|deepseek" \
  macaca/crates/macaca-proto/src/payment_service.rs \
  macaca/crates/macaca-runtime-host/src/payment_admission.rs \
  macaca/crates/macaca-runtime-host/src/payment_service_provider.rs \
  macaca/crates/macaca-sdk/src/payment_client.rs
```

Expected: no matches, except comments that explicitly explain forbidden examples in governance docs.

- [ ] **Step 4: Run GitNexus detect changes before commit**

Run:

```text
detect_changes scope=all repo=agent
```

Expected: risk is LOW or documented. If HIGH/CRITICAL, stop and review affected processes before committing.

## File Responsibility Map

| File | Responsibility |
| --- | --- |
| `macaca/crates/macaca-proto/src/payment_service.rs` | Provider-neutral Payment Service DTOs, command names, snapshots, redacted views. |
| `macaca/crates/macaca-runtime-host/src/payment_admission.rs` | Specification validators for trace, scope, amount, transition, redaction. |
| `macaca/crates/macaca-runtime-host/src/payment_service_provider.rs` | Runtime-host SystemService provider, adapter mediation, store persistence, trace/log emission. |
| `macaca/crates/macaca-sdk/src/payment_client.rs` | SDK `SystemPaymentClient`, service-backed client, unavailable client. |
| `macaca/crates/macaca-web/src/lib.rs` | Host composition root registration only. |
| `macaca/crates/macaca-kernel/src/a2a.rs` | Deprecated compatibility anchor after service path exists. |
| `macaca/docs/route-c-architecture-governance.md` | Payment/A2A ownership rules. |
| `macaca/docs/route-c-serviceization-allowlist.md` | S10 migration debt and expiry conditions. |

## Self-Review Checklist

- Spec coverage: all S10 requirements from the Route C master plan map to a slice.
- Placeholder scan: no task relies on unresolved placeholders or vague "add error handling" instructions.
- Boundary check: Payment/A2A does not move into Web, CLI, Store, Entitlement, Web3, EVM, or provider-specific code.
- Pattern check: Facade, Mediator, Strategy, Command, State, Memento, Observer, Adapter/Bridge, Null Object, and Specification are explicitly assigned.
- Trace/audit check: every mutating command requires `TraceContext`; every lifecycle node logs bounded identifiers.
- Security check: no plan step allows secrets, keys, credentials, raw provider payloads, prompt bodies, or encrypted payload in logs/trace/snapshots.
- Compatibility check: existing kernel A2A tests remain valid; old APIs are deprecated, not deleted.
