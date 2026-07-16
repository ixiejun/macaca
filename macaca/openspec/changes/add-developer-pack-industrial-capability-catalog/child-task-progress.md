# Child Pack Task Progress Audit

## Purpose

This audit file records evidence for the 74 `add-pack-*` child proposal
checklists. It exists to prevent the umbrella catalog work from being confused
with the much larger child-pack implementation work.

## Current Evidence Snapshot

- Child proposal directories: 74.
- Child `tasks.md` files: 74.
- Total child checklist items after the latest verified updates: 3,258.
- Verified complete checklist items: 1,854.
- Remaining unchecked checklist items: 1,404.

## Verified Completed Category

The following category has been marked complete where the task text only
required re-reading the stable governance documents and the umbrella catalog
proposal:

- `1.1 Re-read architecture governance, microkernel boundaries,
  serviceization allowlist, design-pattern guidance/OpenSpec rules, and the
  industrial catalog umbrella proposal before implementation.`

Evidence inspected in this working session:

- `docs/macaca-os-architecture-governance.md`
- `docs/macaca-os-microkernel-boundaries.md`
- `docs/macaca-os-serviceization-allowlist.md`
- `docs/design_patterns.md`
- `openspec/AGENTS.md`
- `openspec/changes/add-developer-pack-industrial-capability-catalog/proposal.md`
- `openspec/changes/add-developer-pack-industrial-capability-catalog/design.md`
- `openspec/changes/add-developer-pack-industrial-capability-catalog/tasks.md`

The `this child proposal` reading task has also been marked complete for the
commerce and device families after reading their local `proposal.md`,
`design.md`, and `tasks.md` files:

- `add-pack-commerce-cart`
- `add-pack-commerce-catalog`
- `add-pack-commerce-entitlement`
- `add-pack-commerce-order`
- `add-pack-commerce-payment-intent`
- `add-pack-commerce-receipt`
- `add-pack-device-camera`
- `add-pack-device-foreground-background-host`
- `add-pack-device-local-files`
- `add-pack-device-notifications`
- `add-pack-device-sensors`

The `this child proposal` reading task has also been marked complete for the
remaining finance, identity, location, and workflow child proposals after
checking their local proposal/design/task context:

- `add-pack-finance-accounting`
- `add-pack-finance-invoice`
- `add-pack-finance-portfolio`
- `add-pack-identity-account`
- `add-pack-identity-auth-handoff`
- `add-pack-identity-profile`
- `add-pack-location-place-search`
- `add-pack-location-timezone`
- `add-pack-workflow-schedule`
- `add-pack-workflow-task`

The commerce family has completed its supplier/API research and scope section
for the six child proposals:

- `add-pack-commerce-cart`
- `add-pack-commerce-catalog`
- `add-pack-commerce-entitlement`
- `add-pack-commerce-order`
- `add-pack-commerce-payment-intent`
- `add-pack-commerce-receipt`

Evidence:

- `openspec/changes/add-pack-commerce-cart/research.md`
- `openspec/changes/add-pack-commerce-catalog/research.md`
- `openspec/changes/add-pack-commerce-entitlement/research.md`
- `openspec/changes/add-pack-commerce-order/research.md`
- `openspec/changes/add-pack-commerce-payment-intent/research.md`
- `openspec/changes/add-pack-commerce-receipt/research.md`
- `openspec validate add-pack-commerce-cart --strict`
- `openspec validate add-pack-commerce-catalog --strict`
- `openspec validate add-pack-commerce-entitlement --strict`
- `openspec validate add-pack-commerce-order --strict`
- `openspec validate add-pack-commerce-payment-intent --strict`
- `openspec validate add-pack-commerce-receipt --strict`

The completed commerce checklist items include `1.2` through `1.6` and `5.4`
in each child proposal. Contract, DTO, command, policy, non-unavailable
providers, SDK, trace, audit, replay, quality-gate, and developer-documentation
implementation tasks remain unchecked until directly implemented and verified.

The commerce `5.4` unavailable-provider item is marked complete for the six
commerce child proposals because the shared runtime-host provider now returns a
trace-required, explicit `unavailable` service result for any command without
echoing command payloads or faking success:

- `crates/runtime/macaca-runtime-host/src/domain_pack_service_provider.rs`
- `DomainPackUnavailableSystemServiceProvider`
- `unavailable_domain_pack_provider_registration`
- `cargo test -p macaca-runtime-host domain_pack_service_provider`

Commerce catalog, cart, order, payment-intent, receipt, and entitlement have
also completed provider-neutral descriptor, DTO, command/result DTO,
descriptor-hash, SDK-discovery, and developer-documentation slices. Verified
evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/commerce_common.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/commerce_catalog.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/commerce_catalog_hashes.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/commerce_cart.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/commerce_order.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/commerce_payment_intent.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/commerce_receipt.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/commerce_entitlement.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/commerce_tests.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_commerce_tests.rs`
- `docs/developer-packs/commerce/catalog.md`
- `docs/developer-packs/commerce/cart.md`
- `docs/developer-packs/commerce/order.md`
- `docs/developer-packs/commerce/payment-intent.md`
- `docs/developer-packs/commerce/receipt.md`
- `docs/developer-packs/commerce/entitlement.md`
- `docs/developer-packs/index.md`
- `cargo test -p macaca-proto domain_pack_contract::tests::commerce_tests -- --nocapture`
- `cargo test -p macaca-sdk domain_pack_client::commerce_tests -- --nocapture`
- `cargo test -p macaca-sdk domain_pack_client -- --nocapture`
- `cargo test -p macaca-proto domain_pack_contract::tests -- --nocapture`
- `openspec validate add-pack-commerce-catalog --strict`
- `openspec validate add-pack-commerce-cart --strict`
- `openspec validate add-pack-commerce-order --strict`
- `openspec validate add-pack-commerce-payment-intent --strict`
- `openspec validate add-pack-commerce-receipt --strict`
- `openspec validate add-pack-commerce-entitlement --strict`

The additional completed catalog, cart, and order checklist items are `2.1`
through `2.12`, `3.1` through `3.8`, `6.1`, `6.4` through `6.6`, `8.5`, and
`8.6`. The additional completed payment-intent checklist items are `2.1`
through `2.10`, `3.1` through `3.8`, `6.1`, `6.4` through `6.6`, `8.5`, and
`8.6`. The additional completed receipt checklist items are `2.1` through
`2.14`, `3.1` through `3.10`, `6.1`, `6.4` through `6.6`, `8.6`, and `8.7`.
The additional completed entitlement checklist items are `2.1` through `2.14`,
`3.1` through `3.11`, `6.1`, `6.4` through `6.6`, `8.6`, and `8.7`. The
commerce `6.4` evidence is limited to provider-neutral app-facing examples in
the commerce guides for catalog search/projection/price/availability/mutation,
cart context/line/discount/estimate/handoff, order creation/status/lifecycle,
payment intent action/capture/cancel/status/idempotency, receipt issue/reissue
delivery/verification/correction/audit, and entitlement grant/check/source
sync/state/seat/usage/proof flows. Admission, policy, resource metering,
entitlement checks, concrete providers, mock providers, provider replacement
tests, SDK command-helper builders, runtime trace emission, audit, replay,
redaction gates, and dependency-boundary gates remain unchecked until directly
implemented and verified.

The six AI child proposals have completed the service-runtime binding,
sanitized observability, and trace-replay evidence slices for unavailable
providers:

- `add-pack-ai-llm`
- `add-pack-ai-embedding`
- `add-pack-ai-rerank`
- `add-pack-ai-vision`
- `add-pack-ai-speech`
- `add-pack-ai-model-evaluation`

Verified completed checklist items:

- `4.1` in each AI proposal: descriptor-derived AI unavailable providers are
  bound and called through `ServiceRuntime`; SDK, shell, kernel, and
  application code do not construct these providers in the verified path.
- `6.1` in each AI proposal: runtime events include sanitized lifecycle,
  service-call, admission-decorator, health, snapshot, and unavailable
  evidence, including policy/resource/entitlement/audit decorator names without
  raw prompt, media, or provider payload leakage.
- `6.2` in each AI proposal: every declared command is trace-addressable through
  the canonical service-bus path with accepted, routed, and completed replay
  events.
- `6.3` in each AI proposal: dependency gates prove that kernel, SDK,
  presentation shells, and the generic application framework do not import or
  construct concrete AI domain-pack provider replacement adapters.
- `6.4` in each AI proposal: no-direct-provider-call and canonical execution
  path coverage proves every declared AI command is routed through the service
  runtime and service bus with trace evidence instead of direct provider calls.
- `6.5` in each AI proposal: strict OpenSpec validation, targeted cargo tests,
  dependency-boundary gates, file-size gates, redaction gates, runtime-host
  service-runtime tests, and unified audit replay gates passed for the verified
  AI implementation slice.
- `4.2` in each AI proposal: `ServiceRuntime` now owns generic lifecycle,
  health, snapshot, shutdown, timeout, cancellation, reply-size, stream-frame,
  and runtime-control metadata redaction behavior. The implementation is
  provider-neutral and does not branch on AI pack names, models, providers,
  applications, or business commands.

Evidence:

- `crates/runtime/macaca-runtime-host/src/service_runtime.rs`
- `crates/runtime/macaca-runtime-host/src/service_runtime/call.rs`
- `crates/runtime/macaca-runtime-host/src/service_runtime/control.rs`
- `crates/runtime/macaca-runtime-host/src/service_runtime/health.rs`
- `crates/runtime/macaca-runtime-host/src/service_runtime/support.rs`
- `crates/runtime/macaca-runtime-host/src/service_runtime_error.rs`
- `crates/runtime/macaca-runtime-host/tests/service_runtime_controls.rs`
- `crates/tests/macaca-integration-tests/tests/domain_pack_ai_boundary_gates.rs`
- `crates/tests/macaca-integration-tests/tests/domain_pack_ai_unavailable_provider.rs`
- `cargo fmt -p macaca-runtime-host -p macaca-integration-tests`
- `cargo fmt -p macaca-runtime-host`
- `cargo test -p macaca-runtime-host --test service_runtime --test service_runtime_controls -- --nocapture`
- `cargo test -p macaca-runtime-host service_runtime -- --nocapture`
- `cargo test -p macaca-runtime-host domain_pack_provider_replacement -- --nocapture`
- `cargo test -p macaca-integration-tests --test domain_pack_ai_unavailable_provider -- --nocapture`
- `cargo test -p macaca-integration-tests --test domain_pack_ai_boundary_gates -- --nocapture`
- `cargo test -p macaca-integration-tests --test protocol_service_dependency_boundaries -- --nocapture`
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate -- --nocapture`
- `cargo test -p macaca-integration-tests --test sdk_no_provider_construction_gate -- --nocapture`
- `cargo test -p macaca-integration-tests --test kernel_purity_gate -- --nocapture`
- `cargo test -p macaca-integration-tests --test p5_terminal_audit_gates -- --nocapture`
- `cargo test -p macaca-integration-tests --test os_layer_file_size_gate -- --nocapture`
- `cargo test -p macaca-integration-tests --test audit_redaction_terminal_gate -- --nocapture`
- `cargo test -p macaca-integration-tests --test unified_audit_replay_terminal_gate -- --nocapture`
- `cargo test -p macaca-runtime-host service_runtime -- --nocapture`
- `cargo test -p macaca-runtime-host domain_pack_provider_replacement -- --nocapture`
- `openspec validate add-pack-ai-llm --strict`
- `openspec validate add-pack-ai-embedding --strict`
- `openspec validate add-pack-ai-rerank --strict`
- `openspec validate add-pack-ai-vision --strict`
- `openspec validate add-pack-ai-speech --strict`
- `openspec validate add-pack-ai-model-evaluation --strict`
- `openspec validate add-developer-pack-industrial-capability-catalog --strict`

AI `4.2` is now checked for the six AI child proposals because the runtime
control evidence directly covers the previously missing generic timeout,
cancellation, bounded reply output, bounded stream-frame count, lifecycle,
health/snapshot, shutdown, audit event, and raw cancellation-token redaction
behavior. Provider-visible commands are scrubbed of runtime-only control
metadata before decorators, service-bus routing, or provider dispatch, while the
runtime still records sanitized cancel-request and cancel-terminal events.

The device family has completed its supplier/API comparison, boundary, and
GitNexus memo section for the five child proposals:

- `add-pack-device-camera`
- `add-pack-device-foreground-background-host`
- `add-pack-device-local-files`
- `add-pack-device-notifications`
- `add-pack-device-sensors`

Evidence:

- `openspec/changes/add-pack-device-camera/research.md`
- `openspec/changes/add-pack-device-foreground-background-host/research.md`
- `openspec/changes/add-pack-device-local-files/research.md`
- `openspec/changes/add-pack-device-notifications/research.md`
- `openspec/changes/add-pack-device-sensors/research.md`
- `openspec validate add-pack-device-camera --strict`
- `openspec validate add-pack-device-foreground-background-host --strict`
- `openspec validate add-pack-device-local-files --strict`
- `openspec validate add-pack-device-notifications --strict`
- `openspec validate add-pack-device-sensors --strict`

The completed device checklist items are limited to `1.2` through `1.4` and
`4.4` in each child proposal. Contract, DTO, command, policy,
non-mock/non-unavailable providers, SDK, ABI, trace, audit, replay,
boundary-gate, and developer-documentation implementation tasks remain
unchecked until directly implemented and verified.

Device camera, foreground/background host, local-files, notifications, and
sensors have also completed their generic app-facing example slices. Verified
evidence:

- `docs/developer-packs/device/camera.md`
- `docs/developer-packs/device/foreground-background-host.md`
- `docs/developer-packs/device/local-files.md`
- `docs/developer-packs/device/notifications.md`
- `docs/developer-packs/device/sensors.md`
- `openspec validate add-pack-device-camera --strict`
- `openspec validate add-pack-device-foreground-background-host --strict`
- `openspec validate add-pack-device-local-files --strict`
- `openspec validate add-pack-device-notifications --strict`
- `openspec validate add-pack-device-sensors --strict`

The additional completed device checklist items are `5.5` and `7.3` in each
child proposal. These examples remain provider-neutral, use synthetic opaque
identifiers, route application calls through typed commands, and document
sanitized unavailable-provider diagnostics without raw host paths, raw media,
raw samples, push tokens, lifecycle logs, or application-specific behavior.

The finance family has completed its supplier/API research and scope section
for the six child proposals:

- `add-pack-finance-accounting`
- `add-pack-finance-invoice`
- `add-pack-finance-portfolio`
- `add-pack-finance-market-data`
- `add-pack-finance-stock`
- `add-pack-finance-crypto`

Evidence:

- `openspec/changes/add-pack-finance-accounting/research.md`
- `openspec/changes/add-pack-finance-invoice/research.md`
- `openspec/changes/add-pack-finance-portfolio/research.md`
- `openspec/changes/add-pack-finance-market-data/research.md`
- `openspec/changes/add-pack-finance-stock/research.md`
- `openspec/changes/add-pack-finance-crypto/research.md`
- `openspec validate add-pack-finance-accounting --strict`
- `openspec validate add-pack-finance-invoice --strict`
- `openspec validate add-pack-finance-portfolio --strict`
- `openspec validate add-pack-finance-market-data --strict`
- `openspec validate add-pack-finance-stock --strict`
- `openspec validate add-pack-finance-crypto --strict`

The completed finance checklist items are limited to `1.2` through `1.6` for
accounting, invoice, and portfolio, and `1.2` through `1.9` for market data,
stock, and crypto.

Market data, stock, crypto, accounting, portfolio, and invoice have also
completed provider-neutral descriptor, DTO, command/result DTO,
descriptor-hash, SDK-discovery, and developer-documentation slices. Verified
evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_common.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_market_data.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_stock.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_crypto.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_commands.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_hashes.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_model.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_portfolio.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_invoice.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_tests.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_finance_tests.rs`
- `docs/developer-packs/finance/market-data.md`
- `docs/developer-packs/finance/stock.md`
- `docs/developer-packs/finance/crypto.md`
- `docs/developer-packs/finance/accounting.md`
- `docs/developer-packs/finance/portfolio.md`
- `docs/developer-packs/finance/invoice.md`
- `docs/developer-packs/index.md`
- `cargo test -p macaca-proto domain_pack_contract::tests::finance_tests -- --nocapture`
- `cargo test -p macaca-sdk domain_pack_client::finance_tests -- --nocapture`
- `cargo test -p macaca-sdk domain_pack_client -- --nocapture`
- `cargo test -p macaca-proto domain_pack_contract::tests -- --nocapture`
- `openspec validate add-pack-finance-market-data --strict`
- `openspec validate add-pack-finance-stock --strict`
- `openspec validate add-pack-finance-crypto --strict`
- `openspec validate add-pack-finance-accounting --strict`
- `openspec validate add-pack-finance-portfolio --strict`
- `openspec validate add-pack-finance-invoice --strict`

The additional completed market-data, stock, and crypto checklist items are
`2.1` through `2.6`, `5.1`, `5.5`, `5.6`, and `7.1` through `7.6`.
The `5.5` and `5.6` evidence is limited to provider-neutral app-facing
examples in the three finance guides, including synthetic market data, stock,
and crypto flows plus sanitized unavailable, entitlement, license, stale-data,
ambiguity, unsupported-scope, provider-quota, network-denied, and
artifact-denied diagnostics. Admission, policy, provider, non-unavailable
runtime integration, SDK command-helper builders, WASM ABI, trace, audit,
replay, boundary-gate, and quality-gate tasks remain unchecked until directly
implemented and verified. The additional completed
accounting checklist items are `2.1` through `2.11`,
`3.1` through `3.8`, `4.1` through `4.5`, `5.4`, `6.1`, `6.3` through
`6.6`, `8.5`, and `8.6`; provider runtime integration, concrete/mock
providers, trace, audit, replay, redaction, no-direct-provider-call, provider
replacement, dependency-boundary, and final quality gates remain unchecked.
Verified `3.7` evidence includes provider-neutral preflight helpers and tests for
balanced debit/credit totals per currency, bounded currency codes, account
active-state references, required dimensions, tax-code reference shape, period
locks, provider write/report support, idempotency keys, and bounded report
request metadata:

- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_model.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_reports.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_tests.rs`
- `cargo test -p macaca-proto domain_pack_contract::tests::finance_accounting_tests -- --nocapture`
- `cargo test -p macaca-proto domain_pack_contract::tests::finance_tests -- --nocapture`
- `cargo test -p macaca-proto domain_pack_contract::tests -- --nocapture`

Verified `3.8` evidence adds `AccountingBoundedCommandSpec`,
`AccountingExecutionControl`, and `AccountingOutputBound` so ledger, report,
and audit-export commands can validate pagination, cursor shape, async job
metadata, timeout, cancellation reference, row count, byte count, artifact
handles, and export-plan redaction metadata before provider dispatch:

- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_bounds.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_model.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_tests.rs`
- `cargo test -p macaca-proto domain_pack_contract::tests::finance_accounting_tests -- --nocapture`
- `cargo test -p macaca-proto domain_pack_contract::tests -- --nocapture`

Verified `4.1` evidence adds `AccountingDeclarationSpec` so application
declarations are rejected unless they use the five accounting permission scopes
`finance.accounting.read`, `finance.accounting.write`,
`finance.accounting.reconcile`, `finance.accounting.report`, or
`finance.accounting.audit_export`, with bounded tenant, entity, and ledger-book
references:

- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_tests.rs`
- `cargo test -p macaca-proto domain_pack_contract::tests::finance_accounting_tests -- --nocapture`
- `cargo test -p macaca-proto domain_pack_contract::tests -- --nocapture`

Verified `4.2` through `4.5` evidence adds contract-level preflight
Specification objects for policy, approval, resources, entitlement, conflict,
freshness, and typed pre-provider rejection states. This evidence proves the
provider-neutral command contract and SDK/service preflight shape; it does not
claim concrete accounting providers, runtime trace emission, provider
replacement, or no-direct-provider-call gates are complete:

- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_preflight.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_tests.rs`
- `cargo test -p macaca-proto domain_pack_contract::tests::finance_accounting_tests -- --nocapture`
- `cargo test -p macaca-proto domain_pack_contract::tests -- --nocapture`

Verified `8.5` evidence keeps every accounting contract implementation file
below the 500-line constitution and runs the existing OS-layer size gate after
splitting report DTOs from core accounting DTOs:

- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_bounds.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_commands.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_hashes.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_model.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_preflight.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_reports.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/finance_accounting_tests.rs`
- `cargo test -p macaca-integration-tests --test os_layer_file_size_gate -- --nocapture`

Verified `6.3` evidence adds the SDK accounting helper builder. The helper
evaluates provider-neutral accounting preflight and then delegates accepted
commands to the existing `DomainPackServiceCallBuilder`, returning typed
preflight rejection without creating a service command when denied:

- `crates/facade/macaca-sdk/src/domain_pack_accounting_client.rs`
- `crates/facade/macaca-sdk/src/domain_pack_accounting_client_tests.rs`
- `cargo test -p macaca-sdk domain_pack_accounting_client -- --nocapture`
- `cargo test -p macaca-sdk domain_pack_client -- --nocapture`

Verified `5.4` evidence extends the generic domain-pack unavailable provider
with an accounting command matrix test. Every `FINANCE_ACCOUNTING_COMMANDS`
entry returns explicit `unavailable` diagnostics, preserves the command name,
uses the accounting pack/service ids, and does not echo raw ledger/account
payload fields:

- `crates/runtime/macaca-runtime-host/src/domain_pack_provider_replacement.rs`
- `cargo test -p macaca-runtime-host domain_pack_provider_replacement -- --nocapture`

The additional completed portfolio checklist items are
`2.1` through `2.11`, `3.1` through `3.7`, `6.1`, `6.4` through `6.6`, and `8.6`;
analytics execution, consent/policy/admission, providers, trace, audit, replay,
and boundary gates remain unchecked. The additional completed invoice checklist
items are `2.1` through `2.10`, `3.1` through `3.8`, `6.1`, `6.4` through
`6.6`, and `8.6`; lifecycle validation gates, recipient policy runtime gates, providers,
trace, audit, replay, and boundary gates remain unchecked.

The finance accounting, portfolio, and invoice `6.4` evidence is limited to
provider-neutral app-facing examples in the finance guides for chart of
accounts, balanced journal planning/posting, report generation,
reconciliation, positions, allocation, performance, risk summaries, rebalance
intent, draft planning, issuing, delivery, payment-status sync, reminders,
exports, and unsupported/unavailable diagnostics.

The identity family has completed its supplier/API research, scope, boundary,
reuse-inventory, and GitNexus memo section for the five child proposals:

- `add-pack-identity-account`
- `add-pack-identity-profile`
- `add-pack-identity-auth-handoff`
- `add-pack-identity-organization`
- `add-pack-identity-tenant`

Evidence:

- `openspec/changes/add-pack-identity-account/research.md`
- `openspec/changes/add-pack-identity-profile/research.md`
- `openspec/changes/add-pack-identity-auth-handoff/research.md`
- `openspec/changes/add-pack-identity-organization/research.md`
- `openspec/changes/add-pack-identity-tenant/research.md`
- `openspec validate add-pack-identity-account --strict`
- `openspec validate add-pack-identity-profile --strict`
- `openspec validate add-pack-identity-auth-handoff --strict`
- `openspec validate add-pack-identity-organization --strict`
- `openspec validate add-pack-identity-tenant --strict`

The completed identity checklist items are limited to `1.2` through `1.6` for
account, profile, and auth handoff, and `1.2` through `1.5` for organization
and tenant. Contract, DTO, command, admission, policy, provider, SDK, WASM ABI,
trace, audit, replay, boundary-gate, quality-gate, and developer-documentation
implementation tasks remain unchecked until directly implemented and verified.

Account, profile, auth handoff, organization, and tenant have also completed
provider-neutral descriptor, DTO, command/result DTO, SDK-discovery, and
developer-documentation slices. Verified evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/identity_common.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/identity_account.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/identity_profile.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/identity_auth_handoff.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/identity_organization.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/identity_tenant.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/spec.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/identity_tests.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_identity_tests.rs`
- `docs/developer-packs/identity/account.md`
- `docs/developer-packs/identity/profile.md`
- `docs/developer-packs/identity/auth-handoff.md`
- `docs/developer-packs/identity/organization.md`
- `docs/developer-packs/identity/tenant.md`
- `docs/developer-packs/index.md`
- `cargo test -p macaca-proto domain_pack_contract::tests::identity_tests -- --nocapture`
- `cargo test -p macaca-sdk domain_pack_client::identity_tests -- --nocapture`
- `cargo test -p macaca-sdk domain_pack_client -- --nocapture`
- `cargo test -p macaca-proto domain_pack_contract::tests -- --nocapture`
- `openspec validate add-pack-identity-account --strict`
- `openspec validate add-pack-identity-profile --strict`
- `openspec validate add-pack-identity-auth-handoff --strict`
- `openspec validate add-pack-identity-organization --strict`
- `openspec validate add-pack-identity-tenant --strict`

The additional completed account checklist items are `2.1` through `2.11`,
`3.1` through `3.9`, `6.1`, `6.4` through `6.6`, and `8.6`. The additional completed
profile checklist items are `2.1` through `2.11`, `3.1` through `3.8`, `6.1`,
`6.4` through `6.6`, and `8.6`. The additional completed auth-handoff checklist items
are `2.1` through `2.12`, `3.1` through `3.8`, `6.1`, `6.4` through `6.6`, and `8.6`.
The additional completed organization checklist items are `2.1` through `2.6`,
`5.1`, and `7.1` through `7.6`. The additional completed tenant checklist
items are `2.1` through `2.6`, `5.1`, and `7.1` through `7.6`. Admission,
policy, resource metering, entitlement checks, concrete providers, mock
providers, provider replacement tests, SDK command-helper builders, WASM ABI,
runtime trace emission, audit, replay, redaction gates, dependency-boundary
gates, and full quality gates remain unchecked until directly implemented and
verified.

The identity account, profile, and auth-handoff `6.4` evidence is limited to
provider-neutral app-facing examples for account create/read/search/update,
lifecycle, link/unlink, status sync, recovery refs, audit export, profile
field masks, privacy, preferences, avatar refs, profile export, handoff start,
callback verification, token-reference exchange, subject evidence, session
binding, cancellation, audit export, and replay/conflict diagnostics.

Identity organization and tenant have also completed their generic app-facing
example and diagnostic example slices. Verified evidence:

- `docs/developer-packs/identity/organization.md`
- `docs/developer-packs/identity/tenant.md`
- `openspec validate add-pack-identity-organization --strict`
- `openspec validate add-pack-identity-tenant --strict`

The additional completed identity checklist items are `5.5` and `5.6` for
organization and tenant. The examples use typed SDK commands, synthetic
organization/tenant refs, audit/artifact handles, and provider-neutral
diagnostics without provider names, credentials, private profile data, raw
invite tokens, raw config values, raw provider payloads, raw audit logs, or
application business workflows.

The location family has completed its supplier/API research, boundary,
reuse-inventory where required, and GitNexus memo section for the five child
proposals:

- `add-pack-location-maps`
- `add-pack-location-geocode`
- `add-pack-location-route`
- `add-pack-location-place-search`
- `add-pack-location-timezone`

Evidence:

- `openspec/changes/add-pack-location-maps/research.md`
- `openspec/changes/add-pack-location-geocode/research.md`
- `openspec/changes/add-pack-location-route/research.md`
- `openspec/changes/add-pack-location-place-search/research.md`
- `openspec/changes/add-pack-location-timezone/research.md`
- `openspec validate add-pack-location-maps --strict`
- `openspec validate add-pack-location-geocode --strict`
- `openspec validate add-pack-location-route --strict`
- `openspec validate add-pack-location-place-search --strict`
- `openspec validate add-pack-location-timezone --strict`

The completed location checklist items are limited to `1.2` through `1.5` for
maps, geocode, and route, `1.2` through `1.4` for place search, and `1.2`
through `1.4` plus `4.3` for timezone. Contract, DTO, command, admission,
policy, non-mock/non-unavailable providers, SDK, WASM ABI, trace, audit,
replay, attribution/retention gates, quality gates, and developer-documentation
implementation tasks remain unchecked until directly implemented and verified.

Location maps, geocode, route, place-search, and timezone have also completed
their generic app-facing example slices. Verified evidence:

- `docs/developer-packs/location/maps.md`
- `docs/developer-packs/location/geocode.md`
- `docs/developer-packs/location/route.md`
- `docs/developer-packs/location/place-search.md`
- `docs/developer-packs/location/timezone.md`
- `openspec validate add-pack-location-maps --strict`
- `openspec validate add-pack-location-geocode --strict`
- `openspec validate add-pack-location-route --strict`
- `openspec validate add-pack-location-place-search --strict`
- `openspec validate add-pack-location-timezone --strict`

The additional completed location checklist items are `5.5`, `5.6`, `7.2`,
and `7.4` for maps/geocode/route, plus `5.5` and `7.3` for
place-search/timezone. The `7.2` evidence is limited to field-level developer
documentation for command/result DTOs, idempotency, coordinate or query/route
semantics, async/batch behavior, retention, attribution, redaction, approval,
artifact retention, and structured error diagnostics. The examples use
synthetic refs, required/optional declaration behavior, provider-neutral
statuses, retention/attribution/cache/artifact handling, and sanitized
diagnostics without provider names, credentials, private addresses, exact
private coordinates, private routes, raw tiles, raw geometries, raw boundary
data, unbounded batches, provider payloads, or application business workflows.

The workflow family has completed its supplier/API or borrowed-platform-pattern
research, boundary/reuse inventory, and GitNexus memo section for the six child
proposals:

- `add-pack-workflow-task`
- `add-pack-workflow-schedule`
- `add-pack-workflow-approval`
- `add-pack-workflow-delegation`
- `add-pack-workflow-review`
- `add-pack-workflow-recovery`

Evidence:

- `openspec/changes/add-pack-workflow-task/research.md`
- `openspec/changes/add-pack-workflow-schedule/research.md`
- `openspec/changes/add-pack-workflow-approval/research.md`
- `openspec/changes/add-pack-workflow-delegation/research.md`
- `openspec/changes/add-pack-workflow-review/research.md`
- `openspec/changes/add-pack-workflow-recovery/research.md`
- `openspec validate add-pack-workflow-task --strict`
- `openspec validate add-pack-workflow-schedule --strict`
- `openspec validate add-pack-workflow-approval --strict`
- `openspec validate add-pack-workflow-delegation --strict`
- `openspec validate add-pack-workflow-review --strict`
- `openspec validate add-pack-workflow-recovery --strict`

The completed workflow checklist items are limited to `1.2` through `1.4` and
`4.4` in each child proposal. Contract, DTO, command, admission, policy,
non-mock/non-unavailable providers, SDK, WASM ABI, trace, audit, replay,
state-machine, quality-gate, and developer-documentation implementation tasks
remain unchecked until directly implemented and verified.

The communication family has completed provider-neutral descriptor, DTO,
SDK-discovery, and developer-documentation evidence for the five child
proposals:

- `add-pack-communication-email`
- `add-pack-communication-messaging`
- `add-pack-communication-notification`
- `add-pack-communication-inbox`
- `add-pack-communication-calendar`

Evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/communication_common.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/communication_email.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/communication_messaging.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/communication_notification.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/communication_inbox.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/communication_calendar.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/communication_tests.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_communication_tests.rs`
- `docs/developer-packs/communication/email.md`
- `docs/developer-packs/communication/messaging.md`
- `docs/developer-packs/communication/notification.md`
- `docs/developer-packs/communication/inbox.md`
- `docs/developer-packs/communication/calendar.md`
- `cargo test -p macaca-proto domain_pack_contract::tests::communication_tests`
- `cargo test -p macaca-sdk domain_pack_client_communication_tests`
- `cargo test -p macaca-sdk domain_pack_client`
- `openspec validate add-pack-communication-email --strict`
- `openspec validate add-pack-communication-messaging --strict`
- `openspec validate add-pack-communication-notification --strict`
- `openspec validate add-pack-communication-inbox --strict`
- `openspec validate add-pack-communication-calendar --strict`

The completed communication checklist items are limited to descriptor metadata,
provider-neutral DTO/command surfaces, stable descriptor hashes, SDK discovery,
and developer-facing documentation tasks (`2.1` through `2.5`, `5.1`, and
`7.1` through `7.4` in each child proposal). Provider admission,
non-mock/non-unavailable provider implementations, WASM ABI exposure, trace,
audit, replay, boundary-gate, and quality-gate tasks remain unchecked until
directly implemented and verified.

Communication notification, inbox, and calendar have also completed their
generic app-facing example slices. Verified evidence:

- `docs/developer-packs/communication/notification.md`
- `docs/developer-packs/communication/inbox.md`
- `docs/developer-packs/communication/calendar.md`
- `openspec validate add-pack-communication-notification --strict`
- `openspec validate add-pack-communication-inbox --strict`
- `openspec validate add-pack-communication-calendar --strict`

The additional completed communication checklist item is `5.4` for
notification, inbox, and calendar. The examples use synthetic refs, typed
command names, provider-neutral unavailable diagnostics, and explicit redaction
constraints without provider workflows, credentials, provider payloads, full
bodies, attachments, conference secrets, private attendees, or
application-specific notification, triage, or scheduling logic.

The knowledge family has completed provider-neutral descriptor, DTO,
SDK-discovery, compatibility-test, and developer-documentation evidence for the
six child proposals:

- `add-pack-knowledge-search`
- `add-pack-knowledge-retrieval`
- `add-pack-knowledge-document-parsing`
- `add-pack-knowledge-citations`
- `add-pack-knowledge-graph`
- `add-pack-knowledge-summarization`

Evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/knowledge_common.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/knowledge_search.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/knowledge_retrieval.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/knowledge_document_parsing.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/knowledge_citations.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/knowledge_graph.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/knowledge_summarization.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/knowledge_tests.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_knowledge_tests.rs`
- `docs/developer-packs/knowledge/search.md`
- `docs/developer-packs/knowledge/retrieval.md`
- `docs/developer-packs/knowledge/document-parsing.md`
- `docs/developer-packs/knowledge/citations.md`
- `docs/developer-packs/knowledge/graph.md`
- `docs/developer-packs/knowledge/summarization.md`
- `cargo test -p macaca-proto domain_pack_contract::tests::knowledge_tests`
- `cargo test -p macaca-sdk domain_pack_client_knowledge_tests`
- `cargo test -p macaca-sdk domain_pack_client`
- `openspec validate add-pack-knowledge-search --strict`
- `openspec validate add-pack-knowledge-retrieval --strict`
- `openspec validate add-pack-knowledge-document-parsing --strict`
- `openspec validate add-pack-knowledge-citations --strict`
- `openspec validate add-pack-knowledge-graph --strict`
- `openspec validate add-pack-knowledge-summarization --strict`

The completed knowledge checklist items are limited to descriptor metadata,
provider-neutral DTO/command/result surfaces, stable hashes, SDK discovery, and
developer-facing documentation. Search, retrieval, document parsing, and
citations completed tasks `2.1` through `2.5`, `5.1`, and `7.1` through `7.4`.
Graph and summarization completed `2.1` through `2.6`, `5.1`, and `7.1`
through `7.6` because their child proposals require an additional descriptor
and DTO compatibility-test item plus extended developer documentation. Provider
admission, permission enforcement, policy/resource gates, mock or concrete
providers, WASM/application ABI exposure, trace, audit, replay,
canonical-execution-path, dependency-boundary, and quality-gate tasks remain
unchecked until directly implemented and verified.

Knowledge search, retrieval, document parsing, citations, graph, and
summarization have also completed their generic app-facing example slices.
Verified evidence:

- `docs/developer-packs/knowledge/search.md`
- `docs/developer-packs/knowledge/retrieval.md`
- `docs/developer-packs/knowledge/document-parsing.md`
- `docs/developer-packs/knowledge/citations.md`
- `docs/developer-packs/knowledge/graph.md`
- `docs/developer-packs/knowledge/summarization.md`
- `openspec validate add-pack-knowledge-search --strict`
- `openspec validate add-pack-knowledge-retrieval --strict`
- `openspec validate add-pack-knowledge-document-parsing --strict`
- `openspec validate add-pack-knowledge-citations --strict`
- `openspec validate add-pack-knowledge-graph --strict`
- `openspec validate add-pack-knowledge-summarization --strict`

The additional completed knowledge checklist items are `5.4` for search,
retrieval, document-parsing, and citations, plus `5.5` and `5.6` for graph and
summarization. The examples use typed SDK/facade commands, synthetic refs,
bounded handles, and provider-neutral diagnostics without provider names,
credentials, raw queries, raw documents, private corpus content, private graph
values, sensitive queries, private conversation text, model outputs,
domain-specific ontologies, or application-specific workflows.

The office family has completed provider-neutral descriptor, DTO,
SDK-discovery, compatibility-test, and developer-documentation evidence for the
five child proposals:

- `add-pack-office-document`
- `add-pack-office-spreadsheet`
- `add-pack-office-presentation`
- `add-pack-office-pdf`
- `add-pack-office-forms`

Evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/office_common.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/office_document.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/office_spreadsheet.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/office_presentation.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/office_pdf.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/office_forms.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/office_tests.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_office_tests.rs`
- `docs/developer-packs/office/document.md`
- `docs/developer-packs/office/spreadsheet.md`
- `docs/developer-packs/office/presentation.md`
- `docs/developer-packs/office/pdf.md`
- `docs/developer-packs/office/forms.md`
- `docs/developer-packs/index.md`
- `cargo test -p macaca-proto domain_pack_contract::tests::office_tests`
- `cargo test -p macaca-sdk domain_pack_client::office_tests`
- `openspec validate add-pack-office-document --strict`
- `openspec validate add-pack-office-spreadsheet --strict`
- `openspec validate add-pack-office-presentation --strict`
- `openspec validate add-pack-office-pdf --strict`
- `openspec validate add-pack-office-forms --strict`

The completed office checklist items are limited to descriptor metadata,
provider-neutral DTO/command/result surfaces, stable hashes, descriptor and DTO
compatibility tests, SDK discovery, and developer-facing documentation (`2.1`
through `2.6`, `5.1`, and `7.1` through `7.6` in each child proposal).
Provider admission, permission enforcement, policy/resource gates, mock or
concrete providers, WASM/application ABI exposure, trace, audit, replay,
canonical-execution-path, dependency-boundary, and quality-gate tasks remain
unchecked until directly implemented and verified.

Office document, spreadsheet, presentation, PDF, and forms have also completed
the generic app-facing example and diagnostic example slices. Verified
evidence:

- `docs/developer-packs/office/document.md`
- `docs/developer-packs/office/spreadsheet.md`
- `docs/developer-packs/office/presentation.md`
- `docs/developer-packs/office/pdf.md`
- `docs/developer-packs/office/forms.md`
- `openspec validate add-pack-office-document --strict`
- `openspec validate add-pack-office-spreadsheet --strict`
- `openspec validate add-pack-office-presentation --strict`
- `openspec validate add-pack-office-pdf --strict`
- `openspec validate add-pack-office-forms --strict`

The additional completed office checklist items are `5.5` and `5.6` in each
child proposal. The examples use synthetic document/workbook/deck/PDF/form
refs, typed planning/request commands, redacted events, artifact handles, and
provider-neutral diagnostics without provider names, credentials, private
comments, personal data, workbook data, hidden sheet content, private notes,
raw PDF bytes, private keys, respondent data, webhook secrets, raw responses,
raw exports, or workflow-specific conventions.

The media family has completed provider-neutral descriptor, DTO, SDK-discovery,
compatibility-test, and developer-documentation evidence for the five child
proposals:

- `add-pack-media-image`
- `add-pack-media-audio`
- `add-pack-media-video`
- `add-pack-media-transcription`
- `add-pack-media-rendering`

Evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/media_common.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/media_image.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/media_audio.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/media_video.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/media_transcription.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/media_rendering.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/media_tests.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_media_tests.rs`
- `docs/developer-packs/media/image.md`
- `docs/developer-packs/media/audio.md`
- `docs/developer-packs/media/video.md`
- `docs/developer-packs/media/transcription.md`
- `docs/developer-packs/media/rendering.md`
- `docs/developer-packs/index.md`
- `cargo test -p macaca-proto domain_pack_contract::tests::media_tests`
- `cargo test -p macaca-sdk domain_pack_client::media_tests`
- `cargo test -p macaca-sdk domain_pack_client`
- `cargo test -p macaca-proto domain_pack_contract::tests`
- `openspec validate add-pack-media-image --strict`
- `openspec validate add-pack-media-audio --strict`
- `openspec validate add-pack-media-video --strict`
- `openspec validate add-pack-media-transcription --strict`
- `openspec validate add-pack-media-rendering --strict`

The completed media checklist items are limited to descriptor metadata,
provider-neutral DTO/command/result surfaces, stable hashes, descriptor and DTO
compatibility tests, SDK discovery, and developer-facing documentation (`2.1`
through `2.6`, `5.1`, and `7.1` through `7.6` in each child proposal).
Provider admission, permission enforcement, policy/resource gates, mock or
concrete providers, WASM/application ABI exposure, trace, audit, replay,
canonical-execution-path, dependency-boundary, and quality-gate tasks remain
unchecked until directly implemented and verified.

Media image, audio, video, transcription, and rendering have also completed the
generic app-facing example and diagnostic example slices. Verified evidence:

- `docs/developer-packs/media/image.md`
- `docs/developer-packs/media/audio.md`
- `docs/developer-packs/media/video.md`
- `docs/developer-packs/media/transcription.md`
- `docs/developer-packs/media/rendering.md`
- `openspec validate add-pack-media-image --strict`
- `openspec validate add-pack-media-audio --strict`
- `openspec validate add-pack-media-video --strict`
- `openspec validate add-pack-media-transcription --strict`
- `openspec validate add-pack-media-rendering --strict`

The additional completed media checklist items are `5.5` and `5.6` in each
child proposal. The examples use synthetic media/source/job/artifact refs,
typed planning/request commands, opaque artifact handles, and sanitized
provider-neutral diagnostics without provider names, credentials, private
recordings, private images, private videos, private conversations, biometric
signals, raw prompts, raw media bytes, raw transcripts, raw templates, raw
pixels, raw exports, provider payloads, or workflow-specific conventions.

The shared domain-pack descriptor contract has completed stable descriptor
hashing and version compatibility evidence for the proposal tasks whose wording
requires only generic descriptor hashing/version compatibility and generic
descriptor tests:

- `add-pack-ai-llm` tasks `2.4` and `2.5`
- `add-pack-ai-vision` tasks `2.4` and `2.5`
- `add-pack-ai-embedding` tasks `2.4` and `2.5`
- `add-pack-ai-rerank` tasks `2.4` and `2.5`
- `add-pack-ai-speech` tasks `2.4` and `2.5`
- `add-pack-ai-model-evaluation` tasks `2.4` and `2.5`
- `add-pack-workflow-approval` tasks `2.4` and `2.5`
- `add-pack-workflow-delegation` tasks `2.4` and `2.5`
- `add-pack-workflow-review` tasks `2.4` and `2.5`
- `add-pack-workflow-recovery` tasks `2.4` and `2.5`

Evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/model.rs`
  exposes `DomainPackDefinition::stable_descriptor_hash()`.
- `crates/foundation/macaca-proto/src/domain_pack_contract/tests.rs`
  covers deterministic descriptor hashing, descriptor-change hash changes, all
  74 industrial descriptors, and compatibility validation.
- `cargo test -p macaca-proto domain_pack_contract::tests::`
- `openspec validate add-pack-ai-llm --strict`
- `openspec validate add-pack-ai-vision --strict`
- `openspec validate add-pack-ai-embedding --strict`
- `openspec validate add-pack-ai-rerank --strict`
- `openspec validate add-pack-ai-speech --strict`
- `openspec validate add-pack-ai-model-evaluation --strict`
- `openspec validate add-pack-workflow-approval --strict`
- `openspec validate add-pack-workflow-delegation --strict`
- `openspec validate add-pack-workflow-review --strict`
- `openspec validate add-pack-workflow-recovery --strict`

This evidence intentionally does not complete tasks that also require
provider-capability hashing, domain-object hashing, DTO snapshot fixtures,
state-machine fixtures, schema migration fixtures, or provider-specific
compatibility tests.

The shared SDK domain-pack Facade has completed generic command-helper builder
evidence for proposal tasks whose wording requires SDK helpers that only produce
canonical traced service calls and never construct providers:

- `add-pack-ai-llm` task `5.3`
- `add-pack-ai-vision` task `5.3`
- `add-pack-ai-embedding` task `5.3`
- `add-pack-ai-rerank` task `5.3`
- `add-pack-ai-speech` task `5.3`
- `add-pack-ai-model-evaluation` task `5.3`
- `add-pack-workflow-task` task `5.3`
- `add-pack-workflow-schedule` task `5.3`
- `add-pack-workflow-approval` task `5.3`
- `add-pack-workflow-delegation` task `5.3`
- `add-pack-workflow-review` task `5.3`
- `add-pack-workflow-recovery` task `5.3`

Evidence:

- `crates/facade/macaca-sdk/src/domain_pack_client.rs` exposes
  `DomainPackServiceCallBuilder`, which validates non-empty command parts and
  builds through `DomainPackResolveResult::service_call_command()`.
- `crates/facade/macaca-sdk/src/lib.rs` and
  `crates/facade/macaca-sdk/src/system_facade.rs` export the builder from the
  SDK/SystemFacade surface.
- `cargo test -p macaca-sdk domain_pack_client::tests::`
- `openspec validate add-pack-ai-llm --strict`
- `openspec validate add-pack-ai-vision --strict`
- `openspec validate add-pack-ai-embedding --strict`
- `openspec validate add-pack-ai-rerank --strict`
- `openspec validate add-pack-ai-speech --strict`
- `openspec validate add-pack-ai-model-evaluation --strict`
- `openspec validate add-pack-workflow-task --strict`
- `openspec validate add-pack-workflow-schedule --strict`
- `openspec validate add-pack-workflow-approval --strict`
- `openspec validate add-pack-workflow-delegation --strict`
- `openspec validate add-pack-workflow-review --strict`
- `openspec validate add-pack-workflow-recovery --strict`

This evidence intentionally does not complete no-direct-provider-call boundary
gate tasks because those require broader dependency gates and canonical
execution-path coverage across SDK helpers, WASM ABI handlers, app admission,
shells, and runtime dispatch.

The shared domain-pack provider capability report contract has completed
generic provider capability reporting evidence for proposal tasks whose wording
requires discovery to distinguish available, degraded, preview, unavailable,
unsupported, and retired states:

- `add-pack-ai-llm` task `4.3`
- `add-pack-ai-vision` task `4.3`
- `add-pack-ai-embedding` task `4.3`
- `add-pack-ai-rerank` task `4.3`
- `add-pack-ai-speech` task `4.3`
- `add-pack-ai-model-evaluation` task `4.3`
- `add-pack-workflow-approval` task `4.3`
- `add-pack-workflow-delegation` task `4.3`
- `add-pack-workflow-review` task `4.3`
- `add-pack-workflow-recovery` task `4.3`

Evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/model.rs`
  defines `DomainPackProviderCapabilityState` and
  `DomainPackProviderCapabilityReport`.
- `DomainPackProviderSnapshot::capability_report()` maps existing bounded
  snapshots into normalized provider states and fails unknown health values
  closed as unavailable.
- `crates/foundation/macaca-proto/src/domain_pack_contract/mod.rs` and
  `crates/facade/macaca-sdk/src/domain_pack_bridge.rs` export the report types.
- `cargo test -p macaca-proto domain_pack_contract::tests::`
- `cargo test -p macaca-sdk domain_pack_client::tests::`
- `openspec validate add-pack-ai-llm --strict`
- `openspec validate add-pack-ai-vision --strict`
- `openspec validate add-pack-ai-embedding --strict`
- `openspec validate add-pack-ai-rerank --strict`
- `openspec validate add-pack-ai-speech --strict`
- `openspec validate add-pack-ai-model-evaluation --strict`
- `openspec validate add-pack-workflow-approval --strict`
- `openspec validate add-pack-workflow-delegation --strict`
- `openspec validate add-pack-workflow-review --strict`
- `openspec validate add-pack-workflow-recovery --strict`

The shared runtime-host domain-pack provider replacement layer has completed
generic mock/unavailable provider evidence for proposal tasks whose wording only
requires deterministic mock and unavailable providers without domain-specific
provider behavior:

- `add-pack-ai-llm` task `4.4`
- `add-pack-ai-vision` task `4.4`
- `add-pack-ai-embedding` task `4.4`
- `add-pack-ai-rerank` task `4.4`
- `add-pack-ai-speech` task `4.4`
- `add-pack-ai-model-evaluation` task `4.4`
- `add-pack-workflow-task` task `4.4`
- `add-pack-workflow-schedule` task `4.4`
- `add-pack-workflow-approval` task `4.4`
- `add-pack-workflow-delegation` task `4.4`
- `add-pack-workflow-review` task `4.4`
- `add-pack-workflow-recovery` task `4.4`
- `add-pack-device-camera` task `4.4`
- `add-pack-device-foreground-background-host` task `4.4`
- `add-pack-device-local-files` task `4.4`
- `add-pack-device-notifications` task `4.4`
- `add-pack-device-sensors` task `4.4`
- `add-pack-location-timezone` task `4.3`

Evidence:

- `crates/runtime/macaca-runtime-host/src/domain_pack_provider_replacement.rs`
  defines `DomainPackMockSystemServiceProvider` and
  `DomainPackUnavailableSystemServiceProvider`.
- The mock provider returns deterministic, trace-required, bounded `ok`
  envelopes without echoing command payloads.
- The unavailable provider returns deterministic, trace-required, bounded
  `unavailable` envelopes without echoing command payloads or faking success.
- Both providers expose descriptor metadata, health, snapshots, lifecycle logs,
  and registration factories for the normal domain-pack bootstrap boundary.
- `crates/runtime/macaca-runtime-host/src/domain_pack_service_provider.rs`
  re-exports the replacement providers from the package-facing bootstrap module.
- `cargo test -p macaca-runtime-host domain_pack_provider_replacement`
- `cargo test -p macaca-runtime-host domain_pack_service_provider`

This evidence intentionally does not complete communication or knowledge mock
provider tasks that require deterministic source, cursor, corpus, schema,
identifier, retrieval, or format-detection behavior. It also does not complete
typed command DTOs, policy checks, provider descriptor matrices, SDK examples,
WASM ABI, trace/audit schemas, or developer documentation tasks.

The shared domain-pack provider descriptor contract has completed generic
provider descriptor support for proposal tasks whose wording requires descriptor
entries for provider classes and does not require pack-specific provider
commands, SDK discovery matrices, ABI exposure, or conformance suites:

- `add-pack-device-camera` task `4.2`
- `add-pack-device-foreground-background-host` task `4.2`
- `add-pack-device-local-files` task `4.2`
- `add-pack-device-notifications` task `4.2`
- `add-pack-device-sensors` task `4.2`
- `add-pack-workflow-task` task `4.2`
- `add-pack-workflow-schedule` task `4.2`
- `add-pack-location-timezone` task `4.2`

Evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/model.rs`
  defines `DomainPackProviderDescriptor` and adds
  `DomainPackMetadata::provider_descriptors` as descriptor-owned provider
  replacement metadata.
- `crates/foundation/macaca-proto/src/domain_pack_contract/mod.rs` exports the
  provider descriptor DTO from the provider-neutral contract boundary.
- `crates/foundation/macaca-proto/src/domain_pack_contract/provider_descriptor_tests.rs`
  proves descriptor hash visibility and support for the provider class labels
  used by the device, workflow, and timezone child proposals.
- `cargo test -p macaca-proto domain_pack_contract::tests::`
- `openspec validate add-pack-device-camera --strict`
- `openspec validate add-pack-device-foreground-background-host --strict`
- `openspec validate add-pack-device-local-files --strict`
- `openspec validate add-pack-device-notifications --strict`
- `openspec validate add-pack-device-sensors --strict`
- `openspec validate add-pack-workflow-task --strict`
- `openspec validate add-pack-workflow-schedule --strict`
- `openspec validate add-pack-location-timezone --strict`

This evidence intentionally does not complete SDK discovery, WASM/application
ABI exposure, provider conformance, lifecycle state machines, trace/audit event
schemas, no-direct-provider-call gates, or developer documentation tasks.

`add-pack-foundation-filesystem` has completed its supplier/API research and
scope section:

- POSIX/Open Group filesystem operations were summarized.
- Node.js `fs` and `fs/promises` filesystem behavior was summarized.
- WASI filesystem and preopened-root concepts were summarized.
- Web File System / OPFS handle, writable stream, and permission concepts were
  summarized.
- The comparison was converted into Macaca-owned root/path/handle/content/
  metadata/watch/snapshot/provider-capability abstractions.
- Provider-native API leakage was explicitly rejected.
- The GitNexus CRITICAL/HIGH memo-only instruction was recorded for the
  research phase before code-symbol implementation work.

Evidence:

- `openspec/changes/add-pack-foundation-filesystem/research.md`
- `openspec validate add-pack-foundation-filesystem --strict`

`add-pack-foundation-key-value-state` has completed its supplier/API research
and scope section:

- Redis key/value, counter, TTL, transaction, scan, stream, and persistence
  concepts were summarized.
- etcd/Consul revision, compare-and-set, lease, watch, prefix query,
  compaction, blocking-query, and health concepts were summarized.
- Apple UserDefaults and iCloud KVS app-scope, value, quota, sync, and conflict
  concepts were summarized.
- Android SharedPreferences and Jetpack DataStore typed preference,
  transactional update, observable flow, and migration concepts were
  summarized.
- Web Storage and IndexedDB origin, transaction, quota, versioning, and async
  persistence concepts were summarized.
- The comparison was converted into Macaca-owned namespace/key/value/revision/
  TTL/consistency/watch/snapshot/provider-capability abstractions.
- Provider-native API leakage was explicitly rejected.
- The GitNexus CRITICAL/HIGH memo-only instruction was recorded for the
  research phase before code-symbol implementation work.

Evidence:

- `openspec/changes/add-pack-foundation-key-value-state/research.md`
- `openspec validate add-pack-foundation-key-value-state --strict`

`add-pack-foundation-time` has completed its supplier/API research and scope
section:

- Apple Foundation date, calendar, timezone, formatting, and timer concepts were
  summarized.
- Java `java.time` immutable clock/instant/duration/zoned value concepts were
  summarized.
- Android AlarmManager and WorkManager exact/inexact alarm, host restriction,
  retry/backoff, and scheduling-limit concepts were summarized.
- JavaScript Date, Intl.DateTimeFormat, and Temporal explicit instant/plain/
  zoned value concepts were summarized.
- POSIX wall-clock, monotonic clock, resolution, drift, and timer concepts were
  summarized.
- The comparison was converted into Macaca-owned instant/monotonic/duration/
  timezone/calendar/locale/format/timer/deadline/provider-capability
  abstractions.
- Provider-native clock/timer handles were explicitly rejected.
- The GitNexus CRITICAL/HIGH memo-only instruction was recorded for the
  research phase before code-symbol implementation work.

Evidence:

- `openspec/changes/add-pack-foundation-time/research.md`
- `openspec validate add-pack-foundation-time --strict`

`add-pack-foundation-time` has also completed its descriptor,
provider-neutral contract DTO, SDK discovery metadata, and
developer-documentation slice:

- Descriptor metadata for `pack.foundation.time.v1` now includes preview
  stability/availability, service ids, command/result schema names, permission
  scopes, policy template, data-governance metadata, SDK namespace, docs link,
  diagnostics, compatibility ranges, and provider descriptors.
- Command DTOs exist for `time.now`, `time.monotonic_now`, `time.clock_health`,
  `time.duration_between`, `time.add_duration`, `time.convert_timezone`,
  `time.resolve_timezone`, `time.calendar_convert`, `time.format`,
  `time.parse`, `time.create_timer`, `time.cancel_timer`,
  `time.inspect_timer`, and `time.evaluate_deadline`.
- Shared DTOs exist for instants, monotonic instants, duration, timezone,
  calendar, locale, format specs, timer references, deadline specs, clock
  source, exactness hint, provider capability, provider snapshot, result
  envelope/status/error, and stable descriptor hashes.
- Tests prove descriptor discoverability, industrial catalog wiring, serde
  compatibility, stable hash behavior, and bounded unavailable result/snapshot
  DTOs.
- SDK discovery can inspect the time contract metadata without constructing a
  concrete host clock, timezone, locale, or timer provider.
- The developer guide documents manifest declaration, wall-clock versus
  monotonic semantics, permissions, commands, DTO guidance, result/error
  statuses, examples, mock-clock policy, unavailable diagnostics, and provider
  replacement. It is cross-linked from the developer-pack index and descriptor
  SDK metadata.

Evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/foundation_time.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/foundation_time_tests.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/industrial_reference_catalogs.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_tests.rs`
- `docs/developer-packs/foundation/time.md`
- `docs/developer-packs/index.md`
- `cargo test -p macaca-proto domain_pack_contract::tests::foundation_time`
- `cargo test -p macaca-sdk domain_pack_client`
- `openspec validate add-pack-foundation-time --strict`

The completed `add-pack-foundation-time` checklist items are limited to `2.1`
through `2.5`, `5.1`, and `7.1` through `7.4`. Admission/policy/resource
tasks, concrete time service providers, SDK per-command helpers, effective
capability projection, WASM/application execution-path tasks, trace/audit/
replay gates, and no-direct-provider-call gates remain unchecked until directly
implemented and verified.

`add-pack-foundation-random` has completed its supplier/API research and scope
section:

- Web Crypto `getRandomValues` and `randomUUID` secure-context, worker, limit,
  and error concepts were summarized.
- Node.js `crypto.randomBytes`, `randomFill`, `randomInt`, and `randomUUID`
  sync/async and provider-limit concepts were summarized.
- Apple Security `SecRandomCopyBytes` and Randomization Services CSPRNG concepts
  were summarized.
- Android/Java `SecureRandom` and `getInstanceStrong` provider selection and
  strong RNG diagnostic concepts were summarized.
- POSIX/system RNG `getrandom`, `/dev/urandom`, blocking, entropy, and provider
  failure concepts were summarized.
- The comparison was converted into Macaca-owned strength/purpose/bytes/
  integer/token/UUID/seed/stream/health/provider-capability abstractions.
- Insecure PRNG and provider-native RNG handles were explicitly rejected.
- The GitNexus CRITICAL/HIGH memo-only instruction was recorded for the
  research phase before code-symbol implementation work.

Evidence:

- `openspec/changes/add-pack-foundation-random/research.md`
- `openspec validate add-pack-foundation-random --strict`

`add-pack-foundation-random` has also completed its descriptor, provider-neutral
contract DTO, SDK discovery metadata, and developer-documentation slice:

- Descriptor metadata for `pack.foundation.random.v1` now includes stability,
  preview-unavailable availability, service ids, command/result schema names,
  permission scopes, policy template, data-governance metadata, SDK namespace,
  docs link, diagnostics, compatibility ranges, and provider descriptors.
- Command DTOs exist for `random.bytes`, `random.fill`, `random.integer`,
  `random.uuid_v4`, `random.nonce`, `random.token`,
  `random.test_stream_create`, `random.test_stream_bytes`,
  `random.entropy_health`, and `random.provider_capabilities`.
- Shared DTOs exist for strength, purpose, alphabet, output encoding, replay
  policy, seed reference, stream reference, entropy health, provider
  capability, provider snapshot, result envelope/status/error, and stable
  descriptor hashes.
- Tests prove descriptor discoverability, industrial catalog wiring, serde
  compatibility, stable hash behavior, and bounded unavailable result/snapshot
  DTOs.
- SDK discovery can inspect the random contract metadata without constructing a
  concrete provider.
- The developer guide documents manifest declaration, permissions, commands,
  DTO guidance, result/error statuses, examples, deterministic-test policy, and
  provider replacement. It is cross-linked from the developer-pack index and the
  descriptor SDK metadata.

Evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/foundation_random.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/foundation_random_tests.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/industrial_reference_catalogs.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_tests.rs`
- `docs/developer-packs/foundation/random.md`
- `docs/developer-packs/index.md`
- `cargo test -p macaca-proto domain_pack_contract::tests::`
- `cargo test -p macaca-sdk domain_pack_client`
- `openspec validate add-pack-foundation-random --strict`

The completed `add-pack-foundation-random` checklist items are limited to
`2.1` through `2.5`, `5.1`, and `7.1` through `7.4`. Admission/policy/resource
tasks, concrete random service providers, SDK per-command helpers, effective
capability projection, WASM/application execution-path tasks, trace/audit/
replay gates, and no-direct-provider-call gates remain unchecked until directly
implemented and verified.

`add-pack-foundation-config` has completed its supplier/API research and scope
section:

- Kubernetes ConfigMap non-confidential key-value/file, source injection, and
  config/code separation concepts were summarized.
- Spring Boot externalized configuration source, profile, precedence, binding,
  and validation concepts were summarized.
- Twelve-Factor deploy-time config, portability, and separation-from-code
  concepts were summarized.
- Android resource qualifier, alternative resource, and preference concepts were
  summarized.
- Apple bundle/defaults/plist typed value and runtime override concepts were
  summarized.
- The comparison was converted into Macaca-owned key/value/schema/layer/
  selector/source/provenance/watch/validation/provider-capability abstractions.
- Raw secret values, app-specific keys in OS code, and provider-native config
  handles were explicitly rejected.
- The GitNexus CRITICAL/HIGH memo-only instruction was recorded for the
  research phase before code-symbol implementation work.

Evidence:

- `openspec/changes/add-pack-foundation-config/research.md`
- `openspec validate add-pack-foundation-config --strict`

`add-pack-foundation-config` has also completed its descriptor,
provider-neutral contract DTO, SDK discovery metadata, and
developer-documentation slice:

- Descriptor metadata for `pack.foundation.config.v1` now includes preview
  stability/availability, service ids, command/result schema names, permission
  scopes, policy template, data-governance metadata, SDK namespace, docs link,
  diagnostics, compatibility ranges, and provider descriptors.
- Command DTOs exist for `config.describe_schema`, `config.get`,
  `config.get_many`, `config.list_keys`, `config.resolve_effective`,
  `config.validate`, `config.explain_provenance`, `config.watch`,
  `config.reload`, `config.snapshot`, and `config.export_redacted`.
- Shared DTOs exist for config key, typed value reference, schema, layer,
  selector/profile, source reference, provenance, watch event, validation
  report, redaction summary, provider capability, provider snapshot, result
  envelope/status/error, and stable descriptor hashes.
- Tests prove descriptor discoverability, industrial catalog wiring, serde
  compatibility, stable hash behavior, and bounded unavailable result/snapshot
  DTOs without reading real environment variables or raw config values.
- SDK discovery can inspect the config contract metadata without constructing a
  package, workspace, environment, remote, or tenant config provider.
- The developer guide documents manifest declaration, config/code separation,
  schema/key/selector/provenance models, secret-reference boundary, permissions,
  commands, DTO guidance, result/error statuses, examples, unavailable
  diagnostics, and provider replacement. It is cross-linked from the
  developer-pack index and descriptor SDK metadata.

Evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/foundation_config.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/foundation_config_tests.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/industrial_reference_catalogs.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_tests.rs`
- `docs/developer-packs/foundation/config.md`
- `docs/developer-packs/index.md`
- `cargo test -p macaca-proto domain_pack_contract::tests::foundation_config`
- `cargo test -p macaca-sdk domain_pack_client`
- `openspec validate add-pack-foundation-config --strict`

The completed `add-pack-foundation-config` checklist items are limited to
`2.1` through `2.5`, `5.1`, and `7.1` through `7.4`. Admission/policy/resource
tasks, concrete config service providers, SDK per-command helpers, effective
capability projection, WASM/application execution-path tasks, trace/audit/
replay gates, and no-direct-provider-call gates remain unchecked until directly
implemented and verified.

`add-pack-foundation-secrets-reference` has completed its supplier/API research
and scope section:

- AWS Secrets Manager get/batch get, versions/stages, rotation, resource policy,
  and CloudTrail audit concepts were summarized.
- HashiCorp Vault mounts, versioned KV, leases, dynamic secrets, policies,
  metadata, and audit behavior were summarized.
- Kubernetes Secret object reference, environment/volume injection, validation,
  retry, and event diagnostic concepts were summarized.
- Apple Keychain item, access group, accessibility, access control, and
  user/device authentication concepts were summarized.
- Cloud key vault/KMS versioned reference, disabled/destroyed version, access
  policy, rotation, and audit concepts were summarized.
- The comparison was converted into Macaca-owned reference/locator/purpose/
  access-policy/lease/resolution-handle/version-status/audit/provider-capability
  abstractions.
- Raw secret values as ordinary app-facing results were explicitly rejected.
- The GitNexus CRITICAL/HIGH memo-only instruction was recorded for the
  research phase before code-symbol implementation work.

Evidence:

- `openspec/changes/add-pack-foundation-secrets-reference/research.md`
- `openspec validate add-pack-foundation-secrets-reference --strict`

`add-pack-foundation-secrets-reference` has also completed its descriptor,
provider-neutral contract DTO, SDK discovery metadata, and
developer-documentation slice:

- Descriptor metadata for `pack.foundation.secrets.reference.v1` now includes
  preview stability/availability, service ids, command/result schema names,
  permission scopes, policy template, data-governance metadata, SDK namespace,
  docs link, diagnostics, compatibility ranges, and provider descriptors.
- Command DTOs exist for `secrets.create_reference`,
  `secrets.import_reference`, `secrets.inspect_reference`,
  `secrets.list_references`, `secrets.bind_purpose`,
  `secrets.resolve_for_provider`, `secrets.create_lease`,
  `secrets.renew_lease`, `secrets.revoke_lease`,
  `secrets.rotate_reference`, `secrets.version_status`, and
  `secrets.audit_access`.
- Shared DTOs exist for secret reference, external redacted locator, purpose
  binding, access policy, lease reference, provider-only resolution handle,
  version status, audit record, provider capability, provider snapshot, result
  envelope/status/error, and stable descriptor hashes.
- Tests prove descriptor discoverability, industrial catalog wiring, serde
  compatibility, stable hash behavior, and bounded unavailable result/snapshot
  DTOs without creating, resolving, or logging a real secret value.
- SDK discovery can inspect the secret-reference contract metadata without
  constructing AWS, Vault, Kubernetes, Keychain, KMS, or other concrete secret
  providers.
- The developer guide documents the reference-only model, raw-secret
  prohibition, manifest declaration, purpose binding, permissions, commands, DTO
  guidance, result/error statuses, examples, unavailable diagnostics, and
  provider replacement. It is cross-linked from the developer-pack index and
  descriptor SDK metadata.

Evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/foundation_secrets_reference.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/foundation_secrets_reference_tests.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/industrial_reference_catalogs.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_tests.rs`
- `docs/developer-packs/foundation/secrets-reference.md`
- `docs/developer-packs/index.md`
- `cargo test -p macaca-proto domain_pack_contract::tests::foundation_secrets_reference`
- `cargo test -p macaca-sdk domain_pack_client`
- `openspec validate add-pack-foundation-secrets-reference --strict`

The completed `add-pack-foundation-secrets-reference` checklist items are
limited to `2.1` through `2.5`, `5.1`, and `7.1` through `7.4`.
Admission/policy/resource tasks, concrete secret-reference service providers,
provider-side injection, SDK per-command helpers, effective capability
projection, WASM/application execution-path tasks, trace/audit/replay gates,
and no-direct-provider-call gates remain unchecked until directly implemented
and verified.

`add-pack-foundation-session-state` has completed its supplier/API research and
scope section:

- Web `sessionStorage` origin/top-level-context partitioning, page-session
  lifetime, key-value, and isolation concepts were summarized.
- Android SavedStateHandle and save-UI-state process-death restoration concepts
  were summarized.
- Apple UIKit/SwiftUI restoration and `NSUserActivity` app/scene restoration
  concepts were summarized.
- Temporal event history, workflow id/run id, and Continue-As-New handoff
  concepts were summarized.
- Redis/server-backed session store TTL, invalidation, serialization, and
  distributed recovery concepts were summarized.
- The comparison was converted into Macaca-owned session ref/key/value/
  revision/checkpoint/restore/recovery/retention/provider-capability
  abstractions.
- Workflow/task-board state repair, shell-owned recovery semantics, raw secrets,
  provider-native handles, and app-specific state keys were explicitly rejected.
- The GitNexus CRITICAL/HIGH memo-only instruction was recorded for the
  research phase before code-symbol implementation work.

Evidence:

- `openspec/changes/add-pack-foundation-session-state/research.md`
- `openspec validate add-pack-foundation-session-state --strict`

`add-pack-communication-email` has completed its supplier/API research and scope
section:

- SMTP, MIME, and IMAP envelope, RFC message, attachment, folder, flag, and
  delivery-error concepts were summarized.
- Gmail messages, drafts, threads, labels, send, attachments, history ids, watch
  notifications, and OAuth-scope concepts were summarized.
- Microsoft Graph Mail sendMail, messages, drafts, folders, attachments, delta,
  subscriptions, and permission concepts were summarized.
- SendGrid personalizations, templates, substitutions, attachments, sandbox
  mode, categories, and event webhook concepts were summarized.
- Mailgun domain sending, templates, variables, tags, test mode, attachments,
  and delivery event concepts were summarized.
- The comparison was converted into Macaca-owned sender, recipient, envelope,
  body-part, attachment, draft/message/thread, sync-cursor, delivery-state, and
  provider-capability abstractions.
- Provider-native message payloads, raw credentials, full message bodies, raw
  attachments, app-specific templates, and shell-owned delivery semantics were
  explicitly rejected.
- The GitNexus CRITICAL/HIGH memo-only instruction was recorded for the
  research phase before code-symbol implementation work.

Evidence:

- `openspec/changes/add-pack-communication-email/research.md`
- `openspec validate add-pack-communication-email --strict`

`add-pack-communication-messaging` has completed its supplier/API research and
scope section:

- Slack chat/conversation/reaction/event, cursor pagination, and rate-limit
  concepts were summarized.
- Microsoft Graph Teams chat/channel message, reply, subscription, reaction,
  HTML restriction, and permission concepts were summarized.
- Telegram Bot API send/edit/delete/reply markup, parse mode, update, chat id,
  and rate behavior concepts were summarized.
- Discord message, reaction, webhook, attachment, and interaction response
  concepts were summarized.
- Twilio Conversations/SMS participant, message, delivery receipt, webhook,
  phone identity, and carrier-state concepts were summarized.
- The comparison was converted into Macaca-owned conversation, participant,
  sender, content, attachment, message, reaction, cursor, delivery-state, and
  provider-capability abstractions.
- Provider-native chat payloads, tokens, webhook secrets, full conversation
  exports, app-specific bot workflows, and provider-specific routing were
  explicitly rejected.
- The GitNexus CRITICAL/HIGH memo-only instruction was recorded for the
  research phase before code-symbol implementation work.

Evidence:

- `openspec/changes/add-pack-communication-messaging/research.md`
- `openspec validate add-pack-communication-messaging --strict`

`add-pack-communication-calendar` has completed its supplier/API research,
provider-neutral mapping, existing-platform inventory, and GitNexus memo
section:

- Google Calendar events/freebusy/sync/watch concepts were summarized.
- Microsoft Graph calendar/events/delta/subscriptions/findMeetingTimes concepts
  were summarized.
- iCalendar RFC 5545 and CalDAV RFC 4791 interchange, recurrence, ETag, and
  sync concepts were summarized.
- Apple EventKit and Android Calendar Provider local host-calendar concepts were
  summarized.
- The comparison was converted into Macaca-owned calendar source, event,
  instance, recurrence, attendee, reminder, conference, availability, cursor,
  watch, conflict, and iCalendar abstractions.
- Existing generic descriptor, SDK facade, service-call, unavailable,
  scheduler, persistence/checkpoint, trace/audit, and policy-command patterns
  were inventoried as reusable platform support, not as completed calendar
  implementation.
- Provider-native calendar payloads, raw credentials, raw invites, conference
  secrets, shell-owned conflict repair, and application-specific scheduling
  workflows were explicitly rejected.

Evidence:

- `openspec/changes/add-pack-communication-calendar/research.md`
- `openspec validate add-pack-communication-calendar --strict`

`add-pack-communication-inbox` has completed its supplier/API research,
provider-neutral mapping, existing-platform inventory, and GitNexus memo
section:

- Gmail messages/threads/labels/history/watch concepts were summarized.
- Microsoft Graph mail/delta/change notification concepts were summarized.
- IMAP mailbox/UID/flags/search/fetch semantics were summarized.
- Slack/Teams conversation history, event, pagination, and stream concepts were
  summarized.
- Host activity-feed acknowledgement/read/archive concepts were summarized.
- The comparison was converted into Macaca-owned source, cursor, item, thread,
  label, attachment, event, read-state, claim, and sync abstractions.
- Existing generic descriptor, SDK facade, service-call, unavailable,
  persistence/checkpoint, trace/audit, and policy-command patterns were
  inventoried as reusable platform support, not as completed inbox
  implementation.
- Provider-native inbox payloads, raw credentials, raw full bodies, raw
  attachments, shell-owned sync/triage loops, and application-specific CRM or
  support workflows were explicitly rejected.

Evidence:

- `openspec/changes/add-pack-communication-inbox/research.md`
- `openspec validate add-pack-communication-inbox --strict`

`add-pack-communication-notification` has completed its supplier/API research,
provider-neutral mapping, existing-platform inventory, and GitNexus memo
section:

- Apple UserNotifications/APNs permission, scheduling, action, and push concepts
  were summarized.
- Android Notifications and Firebase Cloud Messaging permission, channel,
  action, token/topic, foreground/background, delivery, and quota concepts were
  summarized.
- Web Notifications/Push permission, service-worker, subscription, endpoint,
  key, and action concepts were summarized.
- Windows App Notifications local toast/action/activation concepts were
  summarized.
- The comparison was converted into Macaca-owned message, target, delivery
  channel, schedule, action definition/event, subscription handle, delivery
  status, and provider-capability abstractions.
- Existing generic descriptor, SDK facade, service-call, unavailable,
  scheduler, trace/audit, and policy-command patterns were inventoried as
  reusable platform support, not as completed notification implementation.
- Raw push tokens/endpoints/keys, provider payloads, shell-owned fallback or
  action policy, and application-specific notification copy/campaign logic were
  explicitly rejected.

Evidence:

- `openspec/changes/add-pack-communication-notification/research.md`
- `openspec validate add-pack-communication-notification --strict`

The AI family child proposals listed below have completed their research,
platform-pattern mapping, existing-platform inventory, and GitNexus memo
sections:

- `add-pack-ai-llm`
- `add-pack-ai-embedding`
- `add-pack-ai-rerank`
- `add-pack-ai-vision`
- `add-pack-ai-speech`
- `add-pack-ai-model-evaluation`

Shared evidence across this family:

- Mature provider/platform patterns were mapped into Macaca-owned descriptors,
  permission scopes, policy/resource/entitlement checks, typed service calls,
  sanitized audit events, provider capability reports, and unavailable behavior.
- Existing generic Macaca service descriptors, `SystemFacade` focused clients,
  trace-required service-call execution, unavailable/null-object clients,
  runtime-host composition roots, policy-command specifications, persistence
  snapshots, LLM service surfaces, memory embedding patterns, and conformance
  evaluation patterns were inventoried as reusable platform support.
- The inventory explicitly does not complete AI-pack-specific DTOs, service
  providers, SDK helpers, WASM ABI imports, redaction tests, dependency gates,
  developer documentation, or industrial semantic tests.
- GitNexus CRITICAL/HIGH findings remain memo-only for the research phase; no
  Rust symbol was edited for these six research updates.

Per-pack research evidence:

- `openspec/changes/add-pack-ai-llm/research.md`
- `openspec/changes/add-pack-ai-embedding/research.md`
- `openspec/changes/add-pack-ai-rerank/research.md`
- `openspec/changes/add-pack-ai-vision/research.md`
- `openspec/changes/add-pack-ai-speech/research.md`
- `openspec/changes/add-pack-ai-model-evaluation/research.md`

Validation evidence:

- `openspec validate add-pack-ai-llm --strict`
- `openspec validate add-pack-ai-embedding --strict`
- `openspec validate add-pack-ai-rerank --strict`
- `openspec validate add-pack-ai-vision --strict`
- `openspec validate add-pack-ai-speech --strict`
- `openspec validate add-pack-ai-model-evaluation --strict`

The AI family has also completed its provider-neutral descriptor, DTO,
command/result DTO, descriptor-hash, SDK-discovery, and developer-documentation
slice for the six child proposals:

- `add-pack-ai-llm`
- `add-pack-ai-embedding`
- `add-pack-ai-rerank`
- `add-pack-ai-vision`
- `add-pack-ai-speech`
- `add-pack-ai-model-evaluation`

Verified evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_common.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_llm.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_embedding.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_rerank.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_vision.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_speech.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_model_evaluation.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_tests.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/industrial_reference_catalogs.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_ai_tests.rs`
- `docs/developer-packs/ai/llm.md`
- `docs/developer-packs/ai/embedding.md`
- `docs/developer-packs/ai/rerank.md`
- `docs/developer-packs/ai/vision.md`
- `docs/developer-packs/ai/speech.md`
- `docs/developer-packs/ai/model-evaluation.md`
- `docs/developer-packs/index.md`

Validation evidence:

- `cargo fmt --all --check`
- `cargo test -p macaca-proto domain_pack_contract::tests::ai_tests -- --nocapture`
- `cargo test -p macaca-sdk domain_pack_client::ai_tests -- --nocapture`
- `cargo test -p macaca-proto domain_pack_contract::tests -- --nocapture`
- `cargo test -p macaca-sdk domain_pack_client -- --nocapture`
- `openspec validate add-pack-ai-llm --strict`
- `openspec validate add-pack-ai-embedding --strict`
- `openspec validate add-pack-ai-rerank --strict`
- `openspec validate add-pack-ai-vision --strict`
- `openspec validate add-pack-ai-speech --strict`
- `openspec validate add-pack-ai-model-evaluation --strict`

The completed AI checklist items are limited to provider-neutral command DTOs,
typed result DTOs, descriptor metadata, stable hash evidence, SDK discovery,
developer guide, generic examples, catalog cross-links, and the first
industrial semantic model item (`8.1`) for each AI child proposal. All six AI
child proposals have also completed their provider-neutral specialized
contract-validation items (`8.2` through `8.5`) with reference-only DTO checks
and deterministic tests. Runtime permission/policy/resource/entitlement
enforcement, service providers, trace/audit replay, no-direct-provider-call
gates, dependency-boundary gates, and full quality gates remain unchecked until
concrete implementation evidence exists.

Verified `8.2` through `8.5` AI specialized contract evidence includes:

- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_common.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_llm.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_embedding.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_rerank.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_vision.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_speech.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_model_evaluation.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_contract_validation_tests.rs`
- `cargo test -p macaca-proto domain_pack_contract::tests::ai_contract_validation_tests -- --nocapture`
- `cargo test -p macaca-proto domain_pack_contract::tests -- --nocapture`
- `openspec validate add-pack-ai-embedding --strict`
- `openspec validate add-pack-ai-rerank --strict`
- `openspec validate add-pack-ai-vision --strict`
- `openspec validate add-pack-ai-speech --strict`
- `openspec validate add-pack-ai-model-evaluation --strict`
- `openspec validate add-pack-ai-llm --strict`

The AI family has additionally completed its provider-neutral command preflight
slice for tasks `3.1` through `3.5` in each of the six AI child proposals.
This evidence is intentionally limited to declaration validation, policy
decision validation, resource reservation checks, entitlement/unavailable
diagnostics, host-capability diagnostics, unsupported-command diagnostics,
approval-required behavior before provider dispatch, and mock-dispatch tests
proving rejected preflight paths do not invoke the provider closure. Runtime
service-provider binding, trace/audit replay, dependency-boundary gates, and
canonical execution-path gates remain unchecked until service-layer evidence
exists.

Verified AI preflight evidence includes:

- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_preflight.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_preflight_tests.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_common.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/mod.rs`
- `cargo test -p macaca-proto domain_pack_contract::tests::ai_preflight_tests -- --nocapture`
- `cargo test -p macaca-proto domain_pack_contract::tests -- --nocapture`
- `openspec validate add-pack-ai-llm --strict`
- `openspec validate add-pack-ai-embedding --strict`
- `openspec validate add-pack-ai-rerank --strict`
- `openspec validate add-pack-ai-vision --strict`
- `openspec validate add-pack-ai-speech --strict`
- `openspec validate add-pack-ai-model-evaluation --strict`
- `openspec validate add-developer-pack-industrial-capability-catalog --strict`

The AI family has also completed task `5.2` for all six child proposals through
pack-specific admission evidence. Each AI child pack is resolved from the
industrial reference catalog, declared once as required and once as optional,
and verified to produce unresolved-required blocking behavior or
unresolved-optional degradation with explicit unavailable diagnostics while the
preview provider remains unavailable.

Verified AI admission evidence includes:

- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_tests.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/expansion.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/industrial_reference_catalogs.rs`
- `cargo test -p macaca-proto domain_pack_contract::tests::ai_tests -- --nocapture`
- `cargo test -p macaca-proto domain_pack_contract::tests -- --nocapture`
- `openspec validate add-pack-ai-llm --strict`
- `openspec validate add-pack-ai-embedding --strict`
- `openspec validate add-pack-ai-rerank --strict`
- `openspec validate add-pack-ai-vision --strict`
- `openspec validate add-pack-ai-speech --strict`
- `openspec validate add-pack-ai-model-evaluation --strict`
- `openspec validate add-developer-pack-industrial-capability-catalog --strict`

The knowledge family child proposals listed below have completed their
supplier/API research, provider-neutral mapping, existing-platform inventory,
explicit non-goal coverage where required by the child checklist, and GitNexus
memo sections:

- `add-pack-knowledge-search`
- `add-pack-knowledge-retrieval`
- `add-pack-knowledge-citations`
- `add-pack-knowledge-document-parsing`
- `add-pack-knowledge-graph`
- `add-pack-knowledge-summarization`

Shared evidence across this family:

- Supplier/API notes were recorded for the search, retrieval, citations,
  document parsing, graph, and summarization platforms named in each child
  checklist.
- Supplier concepts were mapped into Macaca-owned DTO and descriptor
  boundaries, including provider capability reports, policy/resource concerns,
  trace/audit redaction needs, and canonical service-call ownership.
- Existing generic Macaca descriptor, `SystemFacade`, trace-required service
  call, unavailable/null-object, policy/resource, persistence snapshot, and
  adjacent pack patterns were inventoried as reusable substrate, not counted as
  completed pack implementations.
- Raw provider payloads, credentials, app-specific workflows, provider-native
  pass-through, unbounded outputs, and shell/application-owned OS semantics were
  explicitly rejected where applicable.
- GitNexus CRITICAL/HIGH findings remain memo-only for the research phase; no
  Rust symbol was edited for these six research updates.

Per-pack research evidence:

- `openspec/changes/add-pack-knowledge-search/research.md`
- `openspec/changes/add-pack-knowledge-retrieval/research.md`
- `openspec/changes/add-pack-knowledge-citations/research.md`
- `openspec/changes/add-pack-knowledge-document-parsing/research.md`
- `openspec/changes/add-pack-knowledge-graph/research.md`
- `openspec/changes/add-pack-knowledge-summarization/research.md`

Validation evidence:

- `openspec validate add-pack-knowledge-search --strict`
- `openspec validate add-pack-knowledge-retrieval --strict`
- `openspec validate add-pack-knowledge-citations --strict`
- `openspec validate add-pack-knowledge-document-parsing --strict`
- `openspec validate add-pack-knowledge-graph --strict`
- `openspec validate add-pack-knowledge-summarization --strict`

The office family child proposals listed below have completed their
supplier/API research, provider-neutral mapping, explicit non-goal coverage,
existing-platform inventory, and GitNexus memo sections:

- `add-pack-office-document`
- `add-pack-office-spreadsheet`
- `add-pack-office-presentation`
- `add-pack-office-pdf`
- `add-pack-office-forms`

Shared evidence across this family:

- Google Workspace, Microsoft Office/Graph/OpenXML, LibreOffice UNO,
  Adobe/PDF.js/PDFium/iText, Typeform, Jotform, and related official provider
  surfaces were mapped into Macaca-owned document, spreadsheet, presentation,
  PDF, and forms abstractions.
- Provider concepts were mapped into serviceized DTOs, command boundaries,
  permission/policy/resource concerns, provider capability reporting,
  unavailable behavior, redaction needs, and replayable trace/audit evidence.
- Provider-native request payloads, package internals, raw files, private
  document/sheet/slide/PDF/form contents, credentials, private keys, unbounded
  exports, application-specific workflows, and provider-specific routing were
  explicitly rejected.
- Existing descriptor, `SystemFacade`, service-call, unavailable/null-object,
  policy/resource, persistence snapshot, file, secrets-reference, media
  rendering, and adjacent pack patterns were inventoried as reusable substrate,
  not counted as completed implementations.
- GitNexus CRITICAL/HIGH findings remain memo-only for the research phase; no
  Rust symbol was edited for these five research updates.

Per-pack research evidence:

- `openspec/changes/add-pack-office-document/research.md`
- `openspec/changes/add-pack-office-spreadsheet/research.md`
- `openspec/changes/add-pack-office-presentation/research.md`
- `openspec/changes/add-pack-office-pdf/research.md`
- `openspec/changes/add-pack-office-forms/research.md`

Validation evidence:

- `openspec validate add-pack-office-document --strict`
- `openspec validate add-pack-office-spreadsheet --strict`
- `openspec validate add-pack-office-presentation --strict`
- `openspec validate add-pack-office-pdf --strict`
- `openspec validate add-pack-office-forms --strict`

The media family child proposals listed below have completed their supplier/API
research, provider-neutral mapping, explicit non-goal coverage,
existing-platform inventory, and GitNexus memo sections:

- `add-pack-media-audio`
- `add-pack-media-image`
- `add-pack-media-rendering`
- `add-pack-media-transcription`
- `add-pack-media-video`

Shared evidence across this family:

- FFmpeg, GStreamer, Web Audio, libsndfile, TTS providers, ImageMagick, libvips,
  Sharp, Cloudinary, image generation/annotation providers, Skia, Cairo,
  Canvas, WebGPU, transcription providers, WebCodecs, MediaConvert, and video
  delivery providers were mapped into Macaca-owned media abstractions.
- Provider concepts were mapped into serviceized DTOs, command boundaries,
  resource and policy concerns, provider capability reporting, unavailable
  behavior, output artifact handles, redaction needs, and replayable trace/audit
  evidence.
- Raw media bytes, raw prompts, raw provider payloads, provider-native
  filtergraphs, transformation URLs, model/voice/preset routing, private visual
  or transcript content, credentials, and unbounded pixel/frame/sample/text
  dumps were explicitly rejected.
- Existing descriptor, `SystemFacade`, service-call, unavailable/null-object,
  policy/resource, persistence snapshot, file, secrets-reference, AI speech/vision,
  browser automation, office PDF, and adjacent media pack patterns were
  inventoried as reusable substrate, not counted as completed implementations.
- GitNexus CRITICAL/HIGH findings remain memo-only for the research phase; no
  Rust symbol was edited for these five research updates.

Per-pack research evidence:

- `openspec/changes/add-pack-media-audio/research.md`
- `openspec/changes/add-pack-media-image/research.md`
- `openspec/changes/add-pack-media-rendering/research.md`
- `openspec/changes/add-pack-media-transcription/research.md`
- `openspec/changes/add-pack-media-video/research.md`

Validation evidence:

- `openspec validate add-pack-media-audio --strict`
- `openspec validate add-pack-media-image --strict`
- `openspec validate add-pack-media-rendering --strict`
- `openspec validate add-pack-media-transcription --strict`
- `openspec validate add-pack-media-video --strict`

The developer family child proposals listed below have completed their
supplier/API research, provider-neutral mapping, explicit non-goal coverage,
existing-platform inventory, and GitNexus memo sections:

- `add-pack-developer-browser-automation`
- `add-pack-developer-ci`
- `add-pack-developer-code`
- `add-pack-developer-design-tools`
- `add-pack-developer-issue-tracker`
- `add-pack-developer-repository`
- `add-pack-developer-terminal`

Shared evidence across this family:

- Playwright, CDP, WebDriver BiDi, Selenium, GitHub Actions, GitLab CI,
  CircleCI, Jenkins, LSP, VS Code APIs, Tree-sitter, CodeQL/SARIF, Figma,
  Photoshop UXP, Penpot/DTCG, GitHub/GitLab/Jira/Linear issue trackers, Git,
  GitHub/GitLab/Bitbucket repository APIs, VS Code Terminal, Node.js
  `child_process`, Python `subprocess`, and Docker Exec were mapped into
  Macaca-owned developer abstractions.
- Provider concepts were mapped into serviceized DTOs, command boundaries,
  permission/policy/resource concerns, provider capability reporting,
  unavailable behavior, artifact/transcript/log handles, redaction needs, and
  replayable trace/audit evidence.
- Raw protocols, raw provider payloads, credentials, cookies/tokens, raw source
  text, raw shell commands, provider query languages, unbounded logs/traces,
  private design/issue/repository content, and provider-specific routing were
  explicitly rejected.
- Existing descriptor, `SystemFacade`, service-call, unavailable/null-object,
  policy/resource, persistence snapshot, file, secrets-reference,
  driver/service, repository/CI/code/issue tracker adjacency, media rendering,
  and browser automation patterns were inventoried as reusable substrate, not
  counted as completed implementations.
- GitNexus CRITICAL/HIGH findings remain memo-only for the research phase; no
  Rust symbol was edited for these seven research updates.

Per-pack research evidence:

- `openspec/changes/add-pack-developer-browser-automation/research.md`
- `openspec/changes/add-pack-developer-ci/research.md`
- `openspec/changes/add-pack-developer-code/research.md`
- `openspec/changes/add-pack-developer-design-tools/research.md`
- `openspec/changes/add-pack-developer-issue-tracker/research.md`
- `openspec/changes/add-pack-developer-repository/research.md`
- `openspec/changes/add-pack-developer-terminal/research.md`

Validation evidence:

- `openspec validate add-pack-developer-browser-automation --strict`
- `openspec validate add-pack-developer-ci --strict`
- `openspec validate add-pack-developer-code --strict`
- `openspec validate add-pack-developer-design-tools --strict`
- `openspec validate add-pack-developer-issue-tracker --strict`
- `openspec validate add-pack-developer-repository --strict`
- `openspec validate add-pack-developer-terminal --strict`

The `add-pack-foundation-session-state` proposal has completed its
provider-neutral contract, descriptor, SDK discovery, and developer-documentation
slice. The completed checklist items are limited to `2.1` through `2.5`, `5.1`,
and `7.1` through `7.4`.

Evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/foundation_session_state.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/foundation_session_state_tests.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/industrial_reference_catalogs.rs`
- `crates/facade/macaca-sdk/src/domain_pack_bridge.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_tests.rs`
- `docs/developer-packs/foundation/session-state.md`
- `docs/developer-packs/index.md`

Validation evidence:

- `openspec validate add-pack-foundation-session-state --strict`
- `cargo test -p macaca-proto domain_pack_contract::tests::foundation_session_state`
- `cargo test -p macaca-sdk domain_pack_client`

Provider runtime, admission/policy/resource, SDK command helper, WASM ABI,
trace/audit/replay, no-direct-provider-call, boundary-gate, and full quality
gate tasks remain unchecked until concrete implementation evidence exists.

The `add-pack-foundation-filesystem` proposal has completed its
provider-neutral contract, descriptor, SDK discovery, and developer-documentation
slice. The completed checklist items are limited to `2.1` through `2.5`, `5.1`,
and `7.1` through `7.4`.

Evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/foundation_filesystem.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/foundation_filesystem_tests.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/industrial_reference_catalogs.rs`
- `crates/facade/macaca-sdk/src/domain_pack_bridge.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_tests.rs`
- `docs/developer-packs/foundation/filesystem.md`
- `docs/developer-packs/index.md`

Validation evidence:

- `openspec validate add-pack-foundation-filesystem --strict`
- `cargo test -p macaca-proto domain_pack_contract::tests::foundation_filesystem`
- `cargo test -p macaca-sdk catalog_client_discovers_foundation_filesystem_contract_metadata`

Provider runtime, admission/policy/resource, SDK command helper, WASM ABI,
trace/audit/replay, no-direct-provider-call, boundary-gate, and full quality
gate tasks remain unchecked until concrete implementation evidence exists.

The `add-pack-foundation-key-value-state` proposal has completed its
provider-neutral contract, descriptor, SDK discovery, and developer-documentation
slice. The completed checklist items are limited to `2.1` through `2.5`, `5.1`,
and `7.1` through `7.4`.

Evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/foundation_key_value_state.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/foundation_key_value_state_tests.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/industrial_reference_catalogs.rs`
- `crates/facade/macaca-sdk/src/domain_pack_bridge.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_tests.rs`
- `docs/developer-packs/foundation/key-value-state.md`
- `docs/developer-packs/index.md`

Validation evidence:

- `openspec validate add-pack-foundation-key-value-state --strict`
- `cargo test -p macaca-proto domain_pack_contract::tests::foundation_key_value_state`
- `cargo test -p macaca-sdk catalog_client_discovers_foundation_key_value_state_contract_metadata`

Provider runtime, admission/policy/resource, SDK command helper, WASM ABI,
trace/audit/replay, no-direct-provider-call, boundary-gate, and full quality
gate tasks remain unchecked until concrete implementation evidence exists.

The location family has completed its provider-neutral descriptor, DTO,
command/result DTO, descriptor-hash, SDK-discovery, and developer-documentation
slice for the five child proposals:

- `add-pack-location-maps`
- `add-pack-location-geocode`
- `add-pack-location-route`
- `add-pack-location-place-search`
- `add-pack-location-timezone`

Verified evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/location_common.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/location_maps.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/location_geocode.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/location_route.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/location_place_search.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/location_timezone.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/location_tests.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/industrial_reference_catalogs.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_location_tests.rs`
- `docs/developer-packs/location/maps.md`
- `docs/developer-packs/location/geocode.md`
- `docs/developer-packs/location/route.md`
- `docs/developer-packs/location/place-search.md`
- `docs/developer-packs/location/timezone.md`
- `docs/developer-packs/index.md`

Validation evidence:

- `cargo fmt --all --check`
- `cargo test -p macaca-proto domain_pack_contract::tests::location_tests -- --nocapture`
- `cargo test -p macaca-sdk domain_pack_client::location_tests -- --nocapture`
- `cargo test -p macaca-proto domain_pack_contract::tests -- --nocapture`
- `cargo test -p macaca-sdk domain_pack_client -- --nocapture`

The completed location checklist items are limited to descriptor/DTO/schema
contract, command/result DTO, stable hash, SDK discovery, developer guide,
supplier mapping in documentation, provider-author conformance guidance, and
catalog cross-link tasks. Admission, runtime policy/resource/entitlement,
provider implementation, mock provider, unavailable provider, SDK command
helper builders, WASM ABI, trace/audit emission, replay tests, redaction gates,
dependency-boundary gates, no-direct-provider-call gates, and full quality gates
remain unchecked until concrete implementation evidence exists.

The device family has completed its provider-neutral descriptor, DTO,
command/result DTO, descriptor-hash, SDK-discovery, and developer-documentation
slice for the five child proposals:

- `add-pack-device-sensors`
- `add-pack-device-camera`
- `add-pack-device-local-files`
- `add-pack-device-notifications`
- `add-pack-device-foreground-background-host`

Verified evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/device_common.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/device_sensors.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/device_camera.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/device_local_files.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/device_notifications.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/device_foreground_background_host.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/device_tests.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/industrial_reference_catalogs.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_device_tests.rs`
- `docs/developer-packs/device/sensors.md`
- `docs/developer-packs/device/camera.md`
- `docs/developer-packs/device/local-files.md`
- `docs/developer-packs/device/notifications.md`
- `docs/developer-packs/device/foreground-background-host.md`
- `docs/developer-packs/index.md`

Validation evidence:

- `cargo fmt --all --check`
- `cargo test -p macaca-proto domain_pack_contract::tests::device_tests -- --nocapture`
- `cargo test -p macaca-sdk domain_pack_client::device_tests -- --nocapture`
- `cargo test -p macaca-proto domain_pack_contract::tests -- --nocapture`
- `cargo test -p macaca-sdk domain_pack_client -- --nocapture`
- `openspec validate add-pack-device-sensors --strict`
- `openspec validate add-pack-device-camera --strict`
- `openspec validate add-pack-device-local-files --strict`
- `openspec validate add-pack-device-notifications --strict`
- `openspec validate add-pack-device-foreground-background-host --strict`

The completed device checklist items are limited to contract/descriptor/DTO,
command/result DTO, stable hash, SDK discovery, developer guide, provider-author
conformance guidance, and catalog cross-link tasks. Runtime permission/policy,
resource/entitlement enforcement, service providers, stream/session/lease state
machines, SDK command helper builders, WASM ABI, trace/audit emission, replay
tests, redaction gates, dependency-boundary gates, no-direct-provider-call
gates, and full quality gates remain unchecked until concrete implementation
evidence exists.

The developer family has completed its provider-neutral descriptor, DTO,
command/result DTO, descriptor-hash, SDK-discovery, and developer-documentation
slice for the seven child proposals:

- `add-pack-developer-code`
- `add-pack-developer-repository`
- `add-pack-developer-ci`
- `add-pack-developer-issue-tracker`
- `add-pack-developer-terminal`
- `add-pack-developer-browser-automation`
- `add-pack-developer-design-tools`

Verified evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/developer_common.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/developer_code.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/developer_repository.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/developer_ci.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/developer_issue_tracker.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/developer_terminal.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/developer_browser_automation.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/developer_design_tools.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/developer_tests.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/industrial_reference_catalogs.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_developer_tests.rs`
- `docs/developer-packs/developer/code.md`
- `docs/developer-packs/developer/repository.md`
- `docs/developer-packs/developer/ci.md`
- `docs/developer-packs/developer/issue-tracker.md`
- `docs/developer-packs/developer/terminal.md`
- `docs/developer-packs/developer/browser-automation.md`
- `docs/developer-packs/developer/design-tools.md`
- `docs/developer-packs/index.md`

Validation evidence:

- `cargo fmt --all --check`
- `cargo test -p macaca-proto domain_pack_contract::tests::developer_tests -- --nocapture`
- `cargo test -p macaca-sdk domain_pack_client::developer_tests -- --nocapture`
- `cargo test -p macaca-proto domain_pack_contract::tests -- --nocapture`
- `cargo test -p macaca-sdk domain_pack_client -- --nocapture`
- `openspec validate add-pack-developer-code --strict`
- `openspec validate add-pack-developer-repository --strict`
- `openspec validate add-pack-developer-ci --strict`
- `openspec validate add-pack-developer-issue-tracker --strict`
- `openspec validate add-pack-developer-terminal --strict`
- `openspec validate add-pack-developer-browser-automation --strict`
- `openspec validate add-pack-developer-design-tools --strict`
- `openspec validate add-developer-pack-industrial-capability-catalog --strict`

The completed developer checklist items are limited to `2.1` through `2.6`,
`5.1`, and `7.1` through `7.6` for each child proposal. Admission, permission,
policy, resource, entitlement, approval, concrete service providers,
unavailable/mock runtime providers, SDK command-helper builders, WASM ABI,
trace/audit emission, replay tests, dependency-boundary gates,
no-direct-provider-call gates, and full quality gates remain unchecked until
direct implementation evidence exists.

Developer code, repository, CI, issue-tracker, terminal, browser-automation,
and design-tools have also completed the generic app-facing example and
diagnostic example slices. Verified evidence:

- `docs/developer-packs/developer/code.md`
- `docs/developer-packs/developer/repository.md`
- `docs/developer-packs/developer/ci.md`
- `docs/developer-packs/developer/issue-tracker.md`
- `docs/developer-packs/developer/terminal.md`
- `docs/developer-packs/developer/browser-automation.md`
- `docs/developer-packs/developer/design-tools.md`
- `openspec validate add-pack-developer-code --strict`
- `openspec validate add-pack-developer-repository --strict`
- `openspec validate add-pack-developer-ci --strict`
- `openspec validate add-pack-developer-issue-tracker --strict`
- `openspec validate add-pack-developer-terminal --strict`
- `openspec validate add-pack-developer-browser-automation --strict`
- `openspec validate add-pack-developer-design-tools --strict`

The additional completed developer checklist items are `5.5` and `5.6` in each
child proposal. The examples use typed command concepts, synthetic refs,
opaque handles, redacted diagnostics, and provider-neutral reason codes without
provider names, credentials, raw source, raw terminal output, real remotes,
private issue comments, raw logs, cookies, screenshots, design assets, or
workflow-specific conventions.

The workflow family has completed its provider-neutral descriptor, DTO,
command/result DTO, descriptor-hash, SDK-discovery, and developer-documentation
slice for the six child proposals:

- `add-pack-workflow-task`
- `add-pack-workflow-schedule`
- `add-pack-workflow-approval`
- `add-pack-workflow-delegation`
- `add-pack-workflow-review`
- `add-pack-workflow-recovery`

Verified evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/workflow_common.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/workflow_task.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/workflow_schedule.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/workflow_approval.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/workflow_delegation.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/workflow_review.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/workflow_recovery.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/workflow_tests.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/industrial_reference_catalogs.rs`
- `crates/facade/macaca-sdk/src/domain_pack_client_workflow_tests.rs`
- `docs/developer-packs/workflow/task.md`
- `docs/developer-packs/workflow/schedule.md`
- `docs/developer-packs/workflow/approval.md`
- `docs/developer-packs/workflow/delegation.md`
- `docs/developer-packs/workflow/review.md`
- `docs/developer-packs/workflow/recovery.md`
- `docs/developer-packs/index.md`

Validation evidence:

- `cargo test -p macaca-proto domain_pack_contract::tests::workflow_tests -- --nocapture`
- `cargo test -p macaca-sdk domain_pack_client::workflow_tests -- --nocapture`

The additional completed workflow task and schedule checklist items are `2.1`
through `2.5`, `5.1`, `5.5`, and `7.1` through `7.4`. The `5.5` evidence is
limited to provider-neutral app-facing examples for task create/enqueue/claim,
heartbeat, progress, checkpoint, complete, fail/retry, cancel, dependencies,
concurrency, history, unavailable diagnostics, and schedule one-shot,
interval, cron, RRULE, preview, pause/resume, fire-due, backfill, misfire,
overlap, task integration, history, and unavailable diagnostics. The additional completed
workflow approval, delegation, review, and recovery checklist items are `2.1`
through `2.3`, `5.1`, `5.4`, and `7.1` through `7.3`. Admission, permission,
policy, resource, entitlement, approval runtime checks, concrete service
providers, provider state machines, unavailable/mock runtime providers, WASM
ABI, trace/audit emission, replay tests, redaction gates, dependency-boundary
gates, no-direct-provider-call gates, and full quality gates remain unchecked
until direct implementation evidence exists.

The foundation family has completed the descriptor-driven SDK command-builder
slice for the seven child proposals:

- `add-pack-foundation-filesystem`
- `add-pack-foundation-key-value-state`
- `add-pack-foundation-config`
- `add-pack-foundation-secrets-reference`
- `add-pack-foundation-session-state`
- `add-pack-foundation-time`
- `add-pack-foundation-random`

Verified completed checklist item:

- `5.2` in each listed foundation proposal: SDK command builders now derive
  every declared command from pack descriptors and build canonical traced
  `ServiceCallCommand` envelopes through `DomainPackResolveResult` and
  `DomainPackServiceCallBuilder`.

Evidence:

- `crates/facade/macaca-sdk/src/domain_pack_command_builder.rs`
- `crates/facade/macaca-sdk/src/domain_pack_command_builder_tests.rs`
- `crates/facade/macaca-sdk/src/lib.rs`
- `crates/facade/macaca-sdk/src/system_facade.rs`
- `cargo fmt -p macaca-sdk`
- `cargo test -p macaca-sdk domain_pack_command_builder -- --nocapture`
- `cargo test -p macaca-sdk domain_pack_client -- --nocapture`

The foundation `5.2` evidence is intentionally limited to SDK command
construction. Preview-unavailable pack descriptors can generate SDK command
catalog metadata, but the negative regression test proves they cannot become
callable until an active composition root projects the pack into effective
service capabilities. Foundation semantic helpers, WASM imports,
app-framework path tests, admission/policy/resource checks, service providers,
trace/audit/replay gates, and full quality gates remain unchecked.

The same descriptor-driven SDK command-builder evidence has also completed the
equivalent command-builder checklist item in every non-AI child proposal that
still had one unchecked after the foundation update. The completed scope covers
the following families:

- Commerce: catalog, cart, order, payment-intent, receipt, entitlement.
- Communication: email, messaging, notification, inbox, calendar.
- Developer: code, repository, CI, issue-tracker, terminal,
  browser-automation, design-tools.
- Device: sensors, camera, local-files, notifications,
  foreground-background-host.
- Finance: market-data, stock, crypto, portfolio, invoice.
- Identity: account, profile, auth-handoff, organization, tenant.
- Knowledge: search, retrieval, document-parsing, citations, graph,
  summarization.
- Location: maps, geocode, route, place-search, timezone.
- Office: document, spreadsheet, presentation, PDF, forms.
- Media: image, audio, video, transcription, rendering.

Verified completed checklist item:

- The pack-specific SDK command-builder item in each listed proposal now has
  direct evidence from
  `industrial_catalog_command_builders_cover_every_sub_pack_descriptor`, which
  iterates all 74 industrial sub-pack descriptors, temporarily projects each
  descriptor as callable in a test-only catalog, and proves every declared
  command builds a traced `ServiceCallCommand` through the same generic SDK
  builder path.

Evidence:

- `crates/facade/macaca-sdk/src/domain_pack_command_builder.rs`
- `crates/facade/macaca-sdk/src/domain_pack_command_builder_tests.rs`
- `cargo fmt -p macaca-sdk`
- `cargo test -p macaca-sdk domain_pack_command_builder -- --nocapture`

This evidence proves only SDK command construction and negative provider
non-construction. It does not complete provider adapters, provider state
machines, admission/policy/resource checks, semantic convenience helpers,
WASM imports, app-framework path tests, trace/audit/replay gates, redaction
gates, or full proposal quality gates.

The generic unavailable-provider slice has completed equivalent deterministic
unavailable-provider checklist items across the packs whose tasks required
explicit unavailable diagnostics without fake success:

- Communication: email and messaging.
- Developer: code, repository, CI, issue-tracker, terminal,
  browser-automation, design-tools.
- Foundation: filesystem, key-value-state, config, secrets-reference,
  session-state, time, random.
- Finance: market-data, stock, crypto, portfolio, invoice.
- Identity: account, profile, auth-handoff, organization, tenant.
- Knowledge: graph and summarization.
- Location: maps, geocode, route, and place-search's mock/unavailable fixture
  item.
- Media: image, audio, video, transcription, rendering.
- Office: document, spreadsheet, presentation, PDF, forms.

Verified evidence:

- `crates/runtime/macaca-runtime-host/src/domain_pack_provider_replacement.rs`
- `crates/runtime/macaca-runtime-host/tests/domain_pack_unavailable_provider_catalog.rs`
- `cargo fmt -p macaca-runtime-host`
- `cargo test -p macaca-runtime-host --test domain_pack_unavailable_provider_catalog -- --nocapture`
- `cargo test -p macaca-runtime-host domain_pack_provider_replacement -- --nocapture`

The unavailable-provider catalog test iterates all 74 industrial sub-pack
descriptors and every declared command. For each command it proves the generic
runtime-host provider requires trace, returns structured `unavailable`, carries
pack/service/command/reason metadata, supports provider-not-installed,
unsupported, disabled, missing-entitlement, and provider-health-failed reason
codes, and does not echo raw payload values. This evidence is limited to mock
and unavailable provider fixtures; it does not complete built-in, plugin, or
remote provider replacement tests.

Workflow approval, delegation, review, and recovery have also completed their
generic `4.2` lifecycle-control task. This completion is based on the same
runtime-wide evidence already used by the AI family: service runtime lifecycle,
health, snapshot, shutdown, timeout, cancellation, output bounds, stream-frame
bounds, and sanitized runtime-control metadata behavior are implemented in
`macaca-runtime-host` and verified by targeted service runtime tests. It does
not complete workflow-specific provider binding, entitlement, approval,
resource, or trace/audit/replay tasks.

The generic required/optional admission slice has completed the equivalent
admission checklist item in child proposals whose task only required
unavailable required declarations to block readiness and optional declarations
to degrade explicitly. Completed families include communication calendar,
inbox, notification; developer; finance market-data, stock, crypto; identity
organization and tenant; knowledge; location maps, geocode, route,
place-search; media; office; and workflow.

Evidence:

- `crates/foundation/macaca-proto/src/domain_pack_contract/expansion.rs`
- `crates/foundation/macaca-proto/src/domain_pack_contract/tests.rs`
- `cargo fmt -p macaca-proto`
- `cargo test -p macaca-proto every_industrial_preview_pack_blocks_required_and_degrades_optional -- --nocapture`

The admission test iterates all 74 preview-unavailable industrial sub-pack
descriptors. For each pack it proves a required declaration lands in
`unresolved_required_packs`, an optional declaration lands in
`unresolved_optional_packs`, services remain empty, unavailable reasons are
recorded, and the effective capability memento remains hash-addressable. This
does not complete tasks that also require denied-state admission, disabled
host-capability admission, stale-dataset policy, or pack-specific permission
validation.

## Explicitly Not Completed Yet

The following categories are still incomplete unless a later entry records
stronger evidence:

- Any remaining research, implementation, validation, or documentation task
  outside the verified reading-only category.
- Supplier/API research tasks, because each needs concrete vendor/API notes and
  a provider-neutral mapping memo.
- DTO, descriptor, schema, policy, admission, service runtime, SDK, WASM, trace,
  audit, replay, unavailable-provider, mock-provider, provider implementation,
  developer documentation, and validation tasks, because those require actual
  implementation evidence and tests.

## Completion Rule

A child task may be checked only after the current worktree contains direct
evidence for that exact task. Broad umbrella completion, similar work in another
pack, or absence of obvious failures is not enough.
