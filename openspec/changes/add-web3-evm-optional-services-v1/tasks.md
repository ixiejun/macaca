## 1. Preparation

- [x] 1.1 Read `docs/superpowers/plans/2026-05-10-s11-web3-evm-optional-module-realization-plan.md`, `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-serviceization-allowlist.md`, `macaca/docs/route-c-architecture-governance.md`, and `macaca/docs/design_patterns.md`.
- [x] 1.2 Review `add-optional-web3-node-v0` and `add-optional-evm-dapp-v0` implementation and confirm S11 builds on them instead of redefining basic Web3/EVM value objects.
- [x] 1.3 Run GitNexus impact before editing existing symbols: `Web3Facade`, `EvmFacade`, `Web3Status`, `SystemFacade`, `ServiceRuntime`, and `ServiceProvider`.
- [x] 1.4 Warn before editing any HIGH or CRITICAL impact symbol.
- [x] 1.5 Confirm every touched Rust file remains under 500 lines; split DTO, admission, provider, adapter, and client logic before adding large code.

## 2. Web3 / EVM Service Proto DTOs

- [x] 2.1 Add `macaca/crates/macaca-proto/src/web3_service.rs` with `WEB3_SERVICE_ID` and stable command names for availability, wallet list, signing request, transaction prepare, chain query, and snapshot.
- [x] 2.2 Add `macaca/crates/macaca-proto/src/evm_service.rs` with `EVM_SERVICE_ID` and stable command names for availability, contract deploy, contract call, contract read, gas estimate, receipt query, event subscription, and snapshot.
- [x] 2.3 Define typed Web3 command/result DTOs using existing `web3.rs` value objects where possible.
- [x] 2.4 Define typed EVM command/result DTOs using existing `evm.rs` value objects where possible.
- [x] 2.5 Define provider descriptor, unavailable diagnostics, mock/dev diagnostics, service snapshot, redacted operation summary, and admission result DTOs.
- [x] 2.6 Add validation helpers for trace-required mutating commands, provider descriptor shape, command bounds, and secret-like metadata rejection.
- [x] 2.7 Add English comments explaining provider-neutral boundaries, command semantics, absent-safe behavior, and redaction rules.
- [x] 2.8 Export `web3_service` and `evm_service` from `macaca/crates/macaca-proto/src/lib.rs`.
- [x] 2.9 Add serde, command-name, unavailable diagnostics, and validation tests.
- [x] 2.10 Run `cargo test -p macaca-proto web3_service evm_service web3 evm`.

## 3. Runtime-Host Web3 Optional Service Provider

- [x] 3.1 Add `macaca/crates/macaca-runtime-host/src/web3_service_provider.rs`.
- [x] 3.2 Define an internal Web3 provider adapter trait for availability, wallet list, signing request admission, transaction preparation, chain query, and snapshot.
- [x] 3.3 Implement `UnavailableWeb3Provider` that returns structured unavailable diagnostics and fails closed for mutating commands.
- [x] 3.4 Implement explicit mock/dev Web3 provider with descriptors marking `mock_only`, `development_only`, and `real_chain=false`.
- [x] 3.5 Implement Web3 admission specifications for trace, capability, provider availability, policy status, command bounds, and redaction.
- [x] 3.6 Implement `Web3SystemServiceProvider` as a `SystemService` over the unavailable/mock adapter strategies.
- [x] 3.7 Emit structured logs for provider start, stop, availability query, provider selection, admission rejection, command completion, snapshot query, and failure nodes.
- [x] 3.8 Ensure logs never include private keys, mnemonics, raw signatures, wallet secrets, provider credentials, raw signed transactions, raw RPC credentials, prompt bodies, package bytes, encrypted payload, or unbounded user input.
- [x] 3.9 Export Web3 provider types from `macaca/crates/macaca-runtime-host/src/lib.rs`.
- [x] 3.10 Run `cargo test -p macaca-runtime-host web3_service_provider service_runtime`.

## 4. Runtime-Host EVM Optional Service Provider

- [x] 4.1 Add `macaca/crates/macaca-runtime-host/src/evm_service_provider.rs`.
- [x] 4.2 Define an internal EVM provider adapter trait for availability, contract deploy/call/read admission, gas estimate, receipt query, event subscription admission, and snapshot.
- [x] 4.3 Implement `UnavailableEvmProvider` that returns structured unavailable diagnostics and fails closed for mutating deploy/call commands.
- [x] 4.4 Implement explicit mock/dev EVM provider with deterministic mock identifiers and descriptors marking `mock_only`, `development_only`, and `real_chain=false`.
- [x] 4.5 Implement EVM admission specifications for trace, capability, provider availability, policy status, command bounds, raw ABI/bytecode redaction, and secret-like metadata rejection.
- [x] 4.6 Implement `EvmSystemServiceProvider` as a `SystemService` over the unavailable/mock adapter strategies.
- [x] 4.7 Emit structured logs for provider start, stop, availability query, provider selection, admission rejection, contract operation completion, gas estimate, receipt query, snapshot query, and failure nodes.
- [x] 4.8 Ensure logs never include private keys, mnemonics, raw signatures, wallet secrets, provider credentials, raw signed transactions, raw RPC credentials, raw ABI payload, raw contract bytecode, prompt bodies, package bytes, encrypted payload, or unbounded user input.
- [x] 4.9 Export EVM provider types from `macaca/crates/macaca-runtime-host/src/lib.rs`.
- [x] 4.10 Run `cargo test -p macaca-runtime-host evm_service_provider service_runtime`.

## 5. SDK Focused Clients

- [x] 5.1 Add `macaca/crates/macaca-sdk/src/web3_client.rs`.
- [x] 5.2 Define `SystemWeb3Client` with availability, wallet list, signing request, transaction prepare, chain query, and snapshot methods.
- [x] 5.3 Implement `ServiceBackedWeb3Client` over `SystemServiceClient` and typed `ServiceCallCommand`.
- [x] 5.4 Implement `UnavailableSystemWeb3Client` that fails closed for mutating Web3 commands and returns structured unavailable diagnostics for read-only calls.
- [x] 5.5 Add `macaca/crates/macaca-sdk/src/evm_client.rs`.
- [x] 5.6 Define `SystemEvmClient` with availability, contract deploy/call/read, gas estimate, receipt query, event subscription, and snapshot methods.
- [x] 5.7 Implement `ServiceBackedEvmClient` over `SystemServiceClient` and typed `ServiceCallCommand`.
- [x] 5.8 Implement `UnavailableSystemEvmClient` that fails closed for mutating EVM commands and returns structured unavailable diagnostics for read-only calls.
- [x] 5.9 Add `web3_client` and `evm_client` exports from `macaca/crates/macaca-sdk/src/lib.rs`.
- [x] 5.10 Add `SystemFacade::web3_client()` and `SystemFacade::evm_client()` without constructing runtime-host providers.
- [x] 5.11 Add SDK tests for service-backed dispatch, unavailable fail-closed behavior, and mock/dev diagnostics visibility.
- [x] 5.12 Run `cargo test -p macaca-sdk web3_client evm_client system_facade`.

## 6. Web Composition Root

- [x] 6.1 Register and start built-in unavailable Web3/EVM services in `macaca/crates/macaca-web/src/lib.rs` using the existing `ServiceRuntime` startup pattern.
- [x] 6.2 Permit mock/dev Web3/EVM providers only through explicit test/dev construction.
- [x] 6.3 Route `macaca/crates/macaca-web/src/web3_status.rs` through SDK focused clients or service snapshots.
- [x] 6.4 Do not add Web-owned chain, wallet, RPC, signing, gas, contract, marketplace, payment, provider special case, or app-specific logic.
- [x] 6.5 Run `cargo test -p macaca-web web3_status web3 evm` or `cargo check -p macaca-web` if focused tests are not present.

## 7. Kernel Compatibility Migration

- [x] 7.1 Mark `Web3Facade` and kernel Web3 null/mock adapter helpers as deprecated with notes pointing to `SystemWeb3Client` through `ServiceRuntime`-backed `SystemFacade`.
- [x] 7.2 Mark `EvmFacade` and kernel EVM null/mock adapter helpers as deprecated with notes pointing to `SystemEvmClient` through `ServiceRuntime`-backed `SystemFacade`.
- [x] 7.3 Preserve existing kernel Web3/EVM behavior and tests.
- [x] 7.4 Run `cargo test -p macaca-kernel web3 evm`.

## 8. Governance

- [x] 8.1 Add a Web3 / EVM Optional Module Service Ownership section to `macaca/docs/route-c-architecture-governance.md`.
- [x] 8.2 Update `macaca/docs/route-c-serviceization-allowlist.md` with S11 migration status and remaining deprecated compatibility anchors.
- [x] 8.3 Update executable dependency boundary allowlist only if new direct dependency edges are introduced.
- [x] 8.4 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.

## 9. Verification

- [x] 9.1 Run `openspec validate add-web3-evm-optional-services-v1 --strict`.
- [x] 9.2 Run `cargo fmt --all --check`.
- [x] 9.3 Run `cargo test -p macaca-proto web3_service evm_service web3 evm`.
- [x] 9.4 Run `cargo test -p macaca-kernel web3 evm`.
- [x] 9.5 Run `cargo test -p macaca-runtime-host web3_service_provider evm_service_provider service_runtime`.
- [x] 9.6 Run `cargo test -p macaca-sdk web3_client evm_client system_facade`.
- [x] 9.7 Run `cargo test -p macaca-web web3_status web3 evm` or `cargo check -p macaca-web`.
- [x] 9.8 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
- [x] 9.9 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 9.10 Run `cargo check --workspace`.
- [x] 9.11 Run hardcode scan over new S11 code for app/workflow/provider/driver/gateway/model/chain/business-specific names.
- [x] 9.12 Run GitNexus `detect_changes` before commit and review affected scope.
