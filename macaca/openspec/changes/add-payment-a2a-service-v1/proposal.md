# Change: Serviceize Payment / A2A Runtime

## Why

Route C requires Payment / A2A to be a replaceable system service, not a kernel-owned payment coordinator. The current A2A Payment v0 baseline defines provider-neutral contracts and a kernel compatibility coordinator, but adapter execution, intent lifecycle orchestration, payment memento persistence, and receipt/proof production must move behind `ServiceRuntime` and SDK clients.

This change implements S10 from `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md` and follows `docs/superpowers/plans/2026-05-10-s10-payment-a2a-serviceization-plan.md`.

## What Changes

- Add provider-neutral Payment Service command/result DTOs for quote, intent creation, policy evaluation, approval, settlement, receipt query, transition query, proof query, and snapshot.
- Add runtime-host Payment Service provider that composes policy, adapter, payment store, lifecycle state validation, and trace/audit logging behind `SystemService`.
- Add SDK `SystemPaymentClient` with service-backed and unavailable implementations, plus a `SystemFacade` accessor.
- Register the built-in local simulated Payment Service from the host composition root without moving payment semantics into Web/CLI.
- Mark kernel A2A coordinator and adapter execution helpers as deprecated compatibility anchors once the service path exists.
- Update Route C governance and allowlist documentation with Payment / A2A service ownership and remaining migration debt.

## Non-Goals

- No real external payment provider, card processor, enterprise billing adapter, wallet, chain, smart contract, Web3, or EVM settlement.
- No marketplace billing UI or subscription business console.
- No Store / Entitlement rule migration; S9 owns package entitlement and commercial authorization.
- No removal of existing kernel A2A Payment v0 contracts or compatibility APIs.
- No application-specific, workflow-specific, provider-specific, driver-specific, gateway-specific, model-specific, chain-specific, or business-specific control flow.

## Impact

- Affected specs:
  - `payment-service`
  - `payment-sdk-client`
  - `payment-consumer-migration`
  - `payment-audit-trace`
- Affected code:
  - `macaca/crates/macaca-proto/src/payment_service.rs`
  - `macaca/crates/macaca-proto/src/lib.rs`
  - `macaca/crates/macaca-runtime-host/src/payment_admission.rs`
  - `macaca/crates/macaca-runtime-host/src/payment_service_provider.rs`
  - `macaca/crates/macaca-runtime-host/src/lib.rs`
  - `macaca/crates/macaca-sdk/src/payment_client.rs`
  - `macaca/crates/macaca-sdk/src/system_facade.rs`
  - `macaca/crates/macaca-sdk/src/lib.rs`
  - `macaca/crates/macaca-web/src/lib.rs`
  - `macaca/crates/macaca-kernel/src/a2a.rs`
  - `macaca/docs/route-c-architecture-governance.md`
  - `macaca/docs/route-c-serviceization-allowlist.md`
- Compatibility:
  - Existing A2A Payment v0 tests and no-network task flows must continue to pass.
  - Existing kernel coordinator APIs remain available but deprecated for new production use.

