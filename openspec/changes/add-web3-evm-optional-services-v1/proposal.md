# Change: Realize Web3 / EVM Optional Module Services

## Why

Route C requires Web3 / EVM to be optional, replaceable system services, not kernel-owned skeleton facades or Web shell behavior. The current v0 baseline defines provider-neutral Web3/EVM contracts and compatibility facades, but service ownership, runtime provider lifecycle, SDK focused clients, trace-required admission, and migration boundaries must now move behind `ServiceRuntime`.

This change implements S11 from `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md` and follows `docs/superpowers/plans/2026-05-10-s11-web3-evm-optional-module-realization-plan.md`.

## What Changes

- Add provider-neutral Web3 Service command/result DTOs and snapshots for availability, wallet list, signing request admission, transaction preparation, chain query, and service snapshot.
- Add provider-neutral EVM Service command/result DTOs and snapshots for availability, contract deploy/call/read admission, gas estimate, receipt query, event subscription admission, and service snapshot.
- Add runtime-host Web3/EVM optional service providers with unavailable and mock/dev adapters, provider descriptors, admission specifications, structured logs, and trace/audit emission.
- Add SDK `SystemWeb3Client` and `SystemEvmClient` with service-backed and unavailable implementations, plus `SystemFacade` accessors.
- Register Web3/EVM optional services from the host composition root while keeping Web as a thin shell and status adapter.
- Mark kernel Web3/EVM facades and adapters as deprecated compatibility anchors once the service path exists.
- Update Route C governance and allowlist documentation with S11 Web3/EVM service ownership and remaining migration debt.

## Non-Goals

- No real chain node, RPC provider, wallet private key, mnemonic, keystore, signing secret, gas payment, or chain transaction broadcast.
- No self-built EVM, Substrate/Frontier adapter, real DApp runtime, chain event indexing, or chain execution proof.
- No chain payment settlement, marketplace billing, Store/Entitlement behavior, or Payment Service adapter.
- No new `macaca-web3` or `macaca-evm` crates in this phase.
- No removal of existing kernel Web3/EVM compatibility APIs.
- No application-specific, workflow-specific, provider-specific, driver-specific, gateway-specific, model-specific, chain-specific, or business-specific control flow.

## Impact

- Affected specs:
  - `web3-service`
  - `evm-service`
  - `web3-evm-sdk-client`
  - `web3-evm-consumer-migration`
  - `web3-evm-audit-trace`
- Affected code:
  - `macaca/crates/macaca-proto/src/web3_service.rs`
  - `macaca/crates/macaca-proto/src/evm_service.rs`
  - `macaca/crates/macaca-proto/src/lib.rs`
  - `macaca/crates/macaca-runtime-host/src/web3_service_provider.rs`
  - `macaca/crates/macaca-runtime-host/src/evm_service_provider.rs`
  - `macaca/crates/macaca-runtime-host/src/lib.rs`
  - `macaca/crates/macaca-sdk/src/web3_client.rs`
  - `macaca/crates/macaca-sdk/src/evm_client.rs`
  - `macaca/crates/macaca-sdk/src/system_facade.rs`
  - `macaca/crates/macaca-sdk/src/lib.rs`
  - `macaca/crates/macaca-web/src/lib.rs`
  - `macaca/crates/macaca-web/src/web3_status.rs`
  - `macaca/crates/macaca-kernel/src/web3.rs`
  - `macaca/crates/macaca-kernel/src/evm.rs`
  - `macaca/docs/route-c-architecture-governance.md`
  - `macaca/docs/route-c-serviceization-allowlist.md`
- Compatibility:
  - Existing Web3/EVM v0 tests and absent-safe behavior must continue to pass.
  - Existing kernel Web3/EVM APIs remain available but deprecated for new production use.
  - Ordinary applications that do not declare Web3/EVM capability must continue to run when Web3/EVM services are unavailable or disabled.
