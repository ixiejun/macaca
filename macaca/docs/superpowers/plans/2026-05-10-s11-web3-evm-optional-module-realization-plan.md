# S11 Web3 / EVM Optional Module 真实化 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 Phase 10/11 已有的 Web3/EVM optional skeleton 推进为 Route C 下真实的 optional module service path：可安装、可缺失、可禁用、可 trace，并确保 kernel 不拥有 Web3/EVM provider execution。

**Architecture:** 采用 additive-first：先新增 provider-neutral Web3/EVM service DTO、runtime-host optional providers、SDK focused clients 和 Web composition root registration，再把旧 kernel Web3/EVM facade 标注 deprecated。Web3/EVM 通过 Facade + Adapter/Bridge + Strategy + Command + Null Object + Observer + Specification 表达可插拔边界。

**Tech Stack:** Rust workspace, `macaca-proto`, `macaca-kernel`, `macaca-runtime-host`, `macaca-sdk`, `macaca-web`, ServiceRuntime, OpenSpec, GitNexus, Route C dependency gate.

---

## Scope

S11 覆盖：

- Web3 Service provider-neutral DTO：availability、wallet list、signing request admission、transaction prepare/admission、chain query、snapshot。
- EVM Service provider-neutral DTO：availability、contract deploy/call/read admission、gas estimate、receipt query、event subscription admission、snapshot。
- Runtime-host optional providers：unavailable provider、mock/dev provider、provider descriptor、policy/admission specification、trace/log emission。
- SDK focused clients：`SystemWeb3Client`、`SystemEvmClient`、service-backed client、unavailable client、`SystemFacade` accessors。
- Web composition root：注册 Web3/EVM optional services，Web 只持有 SDK clients 和 status surface。
- Kernel compatibility：旧 `Web3Facade` / `EvmFacade` / null/mock adapter path 标注 deprecated，保留现有语义和测试锚点。
- Governance docs：更新 Route C architecture governance 与 serviceization allowlist，明确 Web3/EVM optional module ownership 和迁移状态。

S11 不覆盖：

- 真实 chain node、RPC provider、wallet private key、mnemonic、keystore、signing secret、gas payment 或 chain transaction broadcast。
- 自研 EVM、Substrate/Frontier adapter、真实 DApp runtime、chain event indexing。
- 链上 payment settlement、marketplace billing、Payment Service adapter，属于后续 Payment/Web3 integration。
- 新增 `macaca-web3` / `macaca-evm` crates。
- 删除旧 kernel Web3/EVM compatibility APIs。

## Required Governance Inputs

- `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`
- `docs/superpowers/plans/2026-05-10-s11-web3-evm-optional-module-realization-brainstorm.md`
- `docs/superpowers/plans/route-c-microkernel-ecosystem/phase-10-optional-web3-node.md`
- `docs/superpowers/plans/route-c-microkernel-ecosystem/phase-11-optional-evm-dapp.md`
- `openspec/changes/add-optional-web3-node-v0`
- `openspec/changes/add-optional-evm-dapp-v0`
- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/design_patterns.md`
- `macaca/docs/route-c-regression-matrix.md` if present in the checkout when implementation starts.

## Architecture Decision

Use two focused optional services, not one Web3 macro-service:

- `Web3 Service` owns Web3 availability, wallet/signing/transaction/chain-query admission, provider descriptor, and snapshot.
- `EVM Service` owns EVM availability, contract operation admission, gas estimate, receipt query, event subscription admission, and snapshot.
- `Kernel` keeps only service registry primitives, policy/trace primitives, and deprecated compatibility anchors.
- `Runtime-host` owns provider lifecycle, command decode, admission specification, strategy dispatch, and trace/log emission.
- `SDK` owns shell-facing focused clients.
- `Web/CLI/Gateway/Application` must use SDK clients and must not directly call kernel Web3/EVM facade for new production paths.

Design patterns:

- Facade: `SystemWeb3Client`, `SystemEvmClient`, Web3 Service, EVM Service.
- Adapter / Bridge: unavailable/mock/future provider adapters behind provider-neutral service contracts.
- Strategy: network policy, signing policy, fee/gas policy, transaction admission, contract call admission, provider selection.
- Command: every operation enters as typed service command before `ServiceRuntime` dispatch.
- Null Object: unavailable clients/providers preserve absent-safe base OS behavior.
- Observer: trace/audit/log events are emitted for availability, admission, denial, provider selection, and snapshot.
- Specification: centralized capability, trace, policy, redaction, command bounds, and provider descriptor validation.
- Proxy: future local/remote/plugin provider shape stays hidden behind service contract.
- State: service lifecycle and command lifecycle use small canonical states.
- Memento: snapshots and operation summaries are redacted/bounded artifacts, not raw provider payloads.

Rejected alternatives:

- SDK wrapper over kernel Web3/EVM facade only: rejected because it leaves optional module execution in kernel.
- Web3-only serviceization with EVM deferred: rejected as final state because S11 explicitly covers Web3/EVM together; acceptable only as internal slice ordering.
- New `macaca-web3` / `macaca-evm` crates now: rejected for this phase because the project currently prefers existing crate structure and no real provider exists yet.
- Real chain/RPC/wallet/EVM integration now: rejected because S11 is service boundary realization, not chain product implementation.

## Proposed OpenSpec Change

Expected change id:

- `add-web3-evm-optional-services-v1`

Expected artifacts:

- `openspec/changes/add-web3-evm-optional-services-v1/proposal.md`
- `openspec/changes/add-web3-evm-optional-services-v1/design.md`
- `openspec/changes/add-web3-evm-optional-services-v1/tasks.md`
- `openspec/changes/add-web3-evm-optional-services-v1/specs/web3-service/spec.md`
- `openspec/changes/add-web3-evm-optional-services-v1/specs/evm-service/spec.md`
- `openspec/changes/add-web3-evm-optional-services-v1/specs/web3-evm-sdk-client/spec.md`
- `openspec/changes/add-web3-evm-optional-services-v1/specs/web3-evm-consumer-migration/spec.md`
- `openspec/changes/add-web3-evm-optional-services-v1/specs/web3-evm-audit-trace/spec.md`

The proposal must state:

- Web3/EVM optional module execution belongs to runtime-host services, not kernel, Web, CLI, Gateway, Application Framework, Store, Entitlement, Payment, or LLM/Memory/Context services.
- Kernel Web3/EVM APIs remain available only as deprecated compatibility anchors until all consumers migrate.
- Web3/EVM must be absent-safe: base OS and applications without declared Web3/EVM capability continue to run when services are unavailable or disabled.
- Mutating Web3/EVM commands require `TraceContext`, capability admission, policy admission, and provider availability.
- Read-only availability/snapshot/list commands may return unavailable diagnostics without failing base OS startup.
- Logs, trace events, snapshots, DTOs, and mementos must not expose private keys, mnemonics, raw signatures, wallet secrets, provider credentials, raw signed transactions, raw RPC credentials, raw contract bytecode, raw ABI payload, prompt bodies, package bytes, or encrypted payload.
- Mock/dev provider must be visibly marked as non-real-chain and cannot be used as proof of real settlement/execution.
- No application/provider/driver/gateway/model/chain/business-specific name can be hardcoded into control flow.

## Implementation Slices

### Slice S11.1: Impact And Boundary Audit

**Files:**

- Inspect: `openspec/AGENTS.md`
- Inspect: `macaca/crates/macaca-proto/src/web3.rs`
- Inspect: `macaca/crates/macaca-proto/src/evm.rs`
- Inspect: `macaca/crates/macaca-kernel/src/web3.rs`
- Inspect: `macaca/crates/macaca-kernel/src/evm.rs`
- Inspect: `macaca/crates/macaca-runtime-host/src/service_runtime.rs`
- Inspect: `macaca/crates/macaca-runtime-host/src/service_provider.rs`
- Inspect: `macaca/crates/macaca-sdk/src/system_facade.rs`
- Inspect: `macaca/crates/macaca-sdk/src/evm.rs`
- Inspect: `macaca/crates/macaca-web/src/web3_status.rs`
- Inspect: `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries.rs`

- [ ] **Step 1: Read current code and OpenSpec instructions**

Run:

```bash
sed -n '1,220p' openspec/AGENTS.md
sed -n '1,260p' macaca/docs/agent-os-microkernel-boundaries.md
sed -n '1,260p' macaca/docs/route-c-serviceization-allowlist.md
sed -n '1,260p' macaca/docs/route-c-architecture-governance.md
sed -n '1,220p' docs/superpowers/plans/route-c-microkernel-ecosystem/phase-10-optional-web3-node.md
sed -n '1,220p' docs/superpowers/plans/route-c-microkernel-ecosystem/phase-11-optional-evm-dapp.md
```

Expected: docs confirm Web3/EVM are optional system modules and must not become kernel/base OS mandatory dependencies.

- [ ] **Step 2: Run GitNexus impact before editing existing symbols**

Run impact for at least:

```text
impact upstream target=Web3Facade
impact upstream target=EvmFacade
impact upstream target=Web3Status
impact upstream target=SystemFacade
impact upstream target=ServiceRuntime
impact upstream target=ServiceProvider
```

Expected: report direct callers, affected flows, and risk. If HIGH/CRITICAL, stop and warn before editing.

- [ ] **Step 3: Classify current Web3/EVM paths**

Document in OpenSpec design:

```text
macaca-proto/src/web3.rs                         => provider-neutral Web3 value object baseline
macaca-proto/src/evm.rs                          => provider-neutral EVM value object baseline
macaca-kernel/src/web3.rs                        => deprecated compatibility facade/adapters
macaca-kernel/src/evm.rs                         => deprecated compatibility facade/adapters
macaca-runtime-host/src/web3_service_provider.rs => new runtime-host Web3 optional service provider
macaca-runtime-host/src/evm_service_provider.rs  => new runtime-host EVM optional service provider
macaca-sdk/src/web3_client.rs                    => new SDK Web3 focused client
macaca-sdk/src/evm_client.rs                     => new SDK EVM focused client
macaca-web/src/web3_status.rs                    => shell/status adapter only
```

Expected: no new provider/app/workflow/chain hardcode is planned.

### Slice S11.2: OpenSpec Proposal And Delta Specs

**Files:**

- Create: `openspec/changes/add-web3-evm-optional-services-v1/proposal.md`
- Create: `openspec/changes/add-web3-evm-optional-services-v1/design.md`
- Create: `openspec/changes/add-web3-evm-optional-services-v1/tasks.md`
- Create: `openspec/changes/add-web3-evm-optional-services-v1/specs/web3-service/spec.md`
- Create: `openspec/changes/add-web3-evm-optional-services-v1/specs/evm-service/spec.md`
- Create: `openspec/changes/add-web3-evm-optional-services-v1/specs/web3-evm-sdk-client/spec.md`
- Create: `openspec/changes/add-web3-evm-optional-services-v1/specs/web3-evm-consumer-migration/spec.md`
- Create: `openspec/changes/add-web3-evm-optional-services-v1/specs/web3-evm-audit-trace/spec.md`

- [ ] **Step 1: Write proposal**

Proposal must include:

```markdown
# add-web3-evm-optional-services-v1

## Why

Web3/EVM currently exist as optional skeleton facades. Route C requires them to become replaceable optional services owned by runtime-host, while kernel keeps only service registry and compatibility anchors.

## What Changes

- Add provider-neutral Web3/EVM service command DTOs and snapshots.
- Add runtime-host unavailable/mock optional providers.
- Add SDK `SystemWeb3Client` and `SystemEvmClient`.
- Register Web3/EVM optional services from the host composition root.
- Mark kernel Web3/EVM facades and adapters deprecated for production consumers.
- Update Route C governance and allowlist with S11 migration state.

## Non-Goals

- No real chain node, RPC, wallet private key, signing secret, or transaction broadcast.
- No self-built EVM or Substrate/Frontier adapter.
- No chain payment settlement.
- No deletion of existing compatibility APIs.
```

- [ ] **Step 2: Write design**

Design must explicitly cover:

```markdown
## Pattern Choices

- Facade: Web3/EVM Service and SDK focused clients.
- Adapter / Bridge: unavailable/mock/future providers.
- Strategy: network, signing, gas, transaction, contract, provider-selection policies.
- Command: all operations enter as typed commands.
- Null Object: absent-safe unavailable behavior.
- Observer: trace/audit/log events.
- Specification: capability, trace, policy, redaction, command bounds.
- Proxy: future local/remote/plugin provider shape.
- State: service and command lifecycle.
- Memento: redacted snapshots and operation summaries.

## Boundary Rules

Kernel must not execute Web3/EVM provider logic. Web/CLI must not define chain, wallet, gas, signing, or contract semantics. Payment settlement must not be implemented inside Web3/EVM.
```

- [ ] **Step 3: Write delta specs**

Minimum requirements:

```markdown
### Requirement: Web3 Optional Service
Web3 Service SHALL expose provider-neutral availability, wallet list, signing request, transaction prepare, chain query, and snapshot commands.

#### Scenario: Web3 unavailable
- WHEN Web3 Service is unavailable
- THEN availability SHALL return structured unavailable diagnostics
- AND mutating Web3 commands SHALL fail closed before provider execution

### Requirement: EVM Optional Service
EVM Service SHALL expose provider-neutral availability, contract deploy/call/read admission, gas estimate, receipt query, event subscription admission, and snapshot commands.

#### Scenario: EVM mutating command without trace
- WHEN an EVM contract deploy or call command lacks TraceContext
- THEN the service SHALL reject it before provider execution

### Requirement: Web3/EVM Audit Redaction
Web3/EVM services SHALL emit bounded trace/audit records and SHALL NOT include private keys, mnemonics, raw signatures, provider credentials, raw signed transactions, raw ABI, or raw bytecode.

### Requirement: SDK Focused Clients
SDK SHALL expose SystemWeb3Client and SystemEvmClient with service-backed and unavailable implementations.

### Requirement: Compatibility Migration
Existing kernel Web3/EVM APIs SHALL remain available but deprecated, and new production consumers SHALL use runtime-host service clients.
```

- [ ] **Step 4: Validate OpenSpec**

Run:

```bash
openspec validate add-web3-evm-optional-services-v1 --strict
```

Expected: `Change 'add-web3-evm-optional-services-v1' is valid`.

### Slice S11.3: Web3/EVM Service DTOs In `macaca-proto`

**Files:**

- Add: `macaca/crates/macaca-proto/src/web3_service.rs`
- Add: `macaca/crates/macaca-proto/src/evm_service.rs`
- Modify: `macaca/crates/macaca-proto/src/lib.rs`

- [ ] **Step 1: Add service identifiers and command names**

Implement provider-neutral constants:

```rust
pub const WEB3_SERVICE_ID: &str = "macaca.web3";
pub const EVM_SERVICE_ID: &str = "macaca.evm";
```

Expected commands include availability, wallet list, signing request, transaction prepare, chain query, contract deploy/call/read, gas estimate, receipt query, event subscribe, and snapshots.

- [ ] **Step 2: Add request/result DTOs**

DTOs must:

- Carry `TraceContext` for mutating commands.
- Use bounded identifiers and redacted metadata.
- Reuse existing `web3.rs` / `evm.rs` value objects where possible.
- Avoid chain/provider/vendor hardcode in control fields.
- Include provider descriptor and mock/dev diagnostics.

- [ ] **Step 3: Add redaction and validation helpers**

Add small specification helpers:

```text
validate_trace_required
validate_no_secret_like_fields
validate_command_bounds
validate_provider_descriptor
```

Expected: helpers remain provider-neutral and do not inspect application-specific names.

- [ ] **Step 4: Unit tests**

Run:

```bash
cargo test -p macaca-proto web3_service evm_service web3 evm
```

Expected: DTO roundtrips, unavailable diagnostics, and validation helpers pass.

### Slice S11.4: Runtime-host Web3 Optional Service Provider

**Files:**

- Add: `macaca/crates/macaca-runtime-host/src/web3_service_provider.rs`
- Modify: `macaca/crates/macaca-runtime-host/src/lib.rs`
- Add tests: `macaca/crates/macaca-runtime-host/tests/web3_service_provider.rs`

- [ ] **Step 1: Add provider trait and provider descriptor**

Define an internal provider adapter trait for Web3 operations:

```text
availability
list_wallets
request_signing
prepare_transaction
query_chain
snapshot
```

Expected: trait is internal to runtime-host provider module and can be backed by unavailable/mock/future adapters.

- [ ] **Step 2: Implement unavailable provider**

Behavior:

- Availability returns unavailable diagnostics.
- Mutating commands fail closed.
- Read-only list/snapshot return empty bounded views with diagnostics.
- Every call emits log/trace event with unavailable reason.

- [ ] **Step 3: Implement mock/dev provider**

Behavior:

- Enabled only by explicit test/dev construction.
- Descriptor marks `mock_only`, `development_only`, and `real_chain=false`.
- Does not sign real payloads, broadcast transactions, or return real chain proofs.

- [ ] **Step 4: Add admission specification**

Admission must check:

- trace required for mutating commands
- capability/policy availability
- provider enabled
- command bounds
- redaction guarantees

- [ ] **Step 5: Unit tests**

Run:

```bash
cargo test -p macaca-runtime-host web3_service_provider service_runtime
```

Expected: unavailable path, mock path, trace-required rejection, and snapshot pass.

### Slice S11.5: Runtime-host EVM Optional Service Provider

**Files:**

- Add: `macaca/crates/macaca-runtime-host/src/evm_service_provider.rs`
- Modify: `macaca/crates/macaca-runtime-host/src/lib.rs`
- Add tests: `macaca/crates/macaca-runtime-host/tests/evm_service_provider.rs`

- [ ] **Step 1: Add provider trait and descriptor**

Define EVM provider adapter operations:

```text
availability
deploy_contract
call_contract
read_contract
estimate_gas
get_receipt
subscribe_events
snapshot
```

Expected: trait is provider-neutral and does not expose concrete EVM engine/RPC vendor.

- [ ] **Step 2: Implement unavailable provider**

Behavior:

- Availability returns unavailable diagnostics.
- Mutating contract deploy/call fail closed.
- Read/snapshot/receipt return empty unavailable views with diagnostics.

- [ ] **Step 3: Implement mock/dev provider**

Behavior:

- Returns deterministic mock contract/result identifiers.
- Marks all outputs as mock-only and non-real-chain.
- Emits trace/log events for admission, denial, and completion.

- [ ] **Step 4: Add EVM admission specification**

Admission must check:

- trace required for deploy/call
- provider enabled
- policy/capability admission
- command size bounds
- no raw ABI/bytecode in trace/memento

- [ ] **Step 5: Unit tests**

Run:

```bash
cargo test -p macaca-runtime-host evm_service_provider service_runtime
```

Expected: unavailable path, mock path, trace-required rejection, gas estimate, and snapshot pass.

### Slice S11.6: SDK Focused Clients

**Files:**

- Add: `macaca/crates/macaca-sdk/src/web3_client.rs`
- Add: `macaca/crates/macaca-sdk/src/evm_client.rs`
- Modify: `macaca/crates/macaca-sdk/src/system_facade.rs`
- Modify: `macaca/crates/macaca-sdk/src/lib.rs`
- Add tests as appropriate under `macaca/crates/macaca-sdk/tests/`

- [ ] **Step 1: Add `SystemWeb3Client`**

Client responsibilities:

- Build typed Web3 service commands.
- Require trace for mutating calls.
- Decode structured unavailable and denied responses.
- Hide runtime provider concrete type from callers.

- [ ] **Step 2: Add `SystemEvmClient`**

Client responsibilities:

- Build typed EVM service commands.
- Require trace for deploy/call.
- Decode structured unavailable and denied responses.
- Keep mock/dev diagnostics visible.

- [ ] **Step 3: Add `SystemFacade` accessors**

Add:

```text
system.web3()
system.evm()
```

Expected: accessors return focused clients, not kernel facades.

- [ ] **Step 4: Unit tests**

Run:

```bash
cargo test -p macaca-sdk web3_client evm_client system_facade
```

Expected: unavailable client and service-backed command construction pass.

### Slice S11.7: Web Composition Root And Shell Status

**Files:**

- Modify: `macaca/crates/macaca-web/src/lib.rs`
- Modify: `macaca/crates/macaca-web/src/web3_status.rs`
- Add/modify tests under `macaca/crates/macaca-web/tests/` if present.

- [ ] **Step 1: Register optional services from composition root**

Web startup may register built-in unavailable providers by default and mock/dev providers only in explicit test/dev construction.

Expected: Web startup does not require real Web3/EVM provider.

- [ ] **Step 2: Route status through SDK clients**

`web3_status` must consume SDK focused clients or service snapshots.

Expected: Web status does not directly instantiate chain, wallet, RPC, EVM, or kernel provider semantics.

- [ ] **Step 3: Shell tests**

Run:

```bash
cargo test -p macaca-web web3_status web3 evm
```

Expected: absent-safe status and mock/dev diagnostics pass.

### Slice S11.8: Kernel Compatibility Deprecation

**Files:**

- Modify: `macaca/crates/macaca-kernel/src/web3.rs`
- Modify: `macaca/crates/macaca-kernel/src/evm.rs`

- [ ] **Step 1: Mark legacy facades and adapters deprecated**

Deprecated items must include clear replacement guidance:

```text
Use SystemWeb3Client through ServiceRuntime-backed SystemFacade.
Use SystemEvmClient through ServiceRuntime-backed SystemFacade.
```

Expected: old APIs remain available and behavior is not deleted.

- [ ] **Step 2: Keep compatibility tests**

Run:

```bash
cargo test -p macaca-kernel web3 evm
```

Expected: existing behavior remains intact, with deprecation warnings only where compiled by direct users.

### Slice S11.9: Governance And Dependency Gate

**Files:**

- Modify: `macaca/docs/route-c-serviceization-allowlist.md`
- Modify: `macaca/docs/route-c-architecture-governance.md`
- Modify if needed: `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries.rs`

- [ ] **Step 1: Update governance**

Add an S11 section:

```markdown
## S11 Web3 / EVM Optional Module Service Ownership

- Web3/EVM provider execution belongs to runtime-host optional services.
- Kernel Web3/EVM facades are deprecated compatibility anchors.
- Web/CLI/Gateway/Application must use SDK focused clients.
- Real chain/RPC/wallet/EVM provider integration must be optional and cannot become base OS dependency.
```

- [ ] **Step 2: Update allowlist**

Record temporary compatibility exceptions:

```text
macaca-kernel::web3 => deprecated compatibility anchor, replace with SystemWeb3Client
macaca-kernel::evm  => deprecated compatibility anchor, replace with SystemEvmClient
```

- [ ] **Step 3: Run boundary gate**

Run:

```bash
cargo test -p macaca-integration-tests route_c_dependency_boundaries
```

Expected: new service path is allowed; no new kernel/provider hard dependency is introduced.

### Slice S11.10: Full Verification

- [ ] **Step 1: OpenSpec validation**

Run:

```bash
openspec validate add-web3-evm-optional-services-v1 --strict
```

- [ ] **Step 2: Formatting**

Run:

```bash
cargo fmt --all --check
```

- [ ] **Step 3: Focused Rust tests**

Run:

```bash
cargo test -p macaca-proto web3_service evm_service web3 evm
cargo test -p macaca-kernel web3 evm
cargo test -p macaca-runtime-host web3_service_provider evm_service_provider service_runtime
cargo test -p macaca-sdk web3_client evm_client system_facade
cargo test -p macaca-web web3_status web3 evm
cargo test -p macaca-integration-tests route_c_dependency_boundaries
cargo test -p macaca-integration-tests --test route_c_baseline
cargo check --workspace
```

- [ ] **Step 4: GitNexus change detection before commit**

Run:

```bash
npx gitnexus detect-changes -r agent --scope unstaged
```

Expected: affected symbols and flows match S11 Web3/EVM optional service scope.

## Rollback Plan

- Revert runtime-host Web3/EVM provider registration first; unavailable status should keep base OS booting.
- Revert SDK focused clients if no consumer has migrated yet.
- Keep proto DTOs if OpenSpec remains accepted, because they are additive provider-neutral contracts.
- Keep deprecated kernel compatibility APIs intact throughout rollback.

## Done Criteria

- OpenSpec `add-web3-evm-optional-services-v1` is valid.
- Web3/EVM service DTOs exist and are provider-neutral.
- Runtime-host owns unavailable/mock optional providers.
- SDK exposes `SystemWeb3Client` and `SystemEvmClient`.
- Web uses SDK/status snapshots and does not own Web3/EVM semantics.
- Kernel Web3/EVM facade/adapters are deprecated compatibility anchors.
- Route C governance and allowlist document S11 ownership.
- Focused tests, dependency gate, formatting, and workspace check pass.
