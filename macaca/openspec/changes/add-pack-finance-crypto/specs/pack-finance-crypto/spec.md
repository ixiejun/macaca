## ADDED Requirements

### Requirement: Macaca SHALL provide the Finance Crypto Pack as a serviceized capability

Macaca SHALL provide `pack.finance.crypto.v1` as a provider-neutral industrial
pack for provider inspection, crypto asset search, asset metadata, token
reference lookup, chain/network reference lookup, exchange market pair
discovery, quotes, trades, historical bars/candles, snapshots, supply metrics,
market status, optional read-only public address balance diagnostics, oracle/
feed references, freshness diagnostics, attribution, artifact handles,
snapshot, and replay. The pack SHALL be declared by applications, resolved by
application admission and catalog services, and invoked only through typed
service commands owned by the crypto finance service provider.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.finance.crypto.v1` as required and the crypto finance service provider is registered, healthy, entitled, licensed, permissioned, address-policy-admissible, resource-admissible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, policy templates, resource limits, address privacy policy, attribution requirements, freshness classes, health, compatibility, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider credentials, private keys, seed phrases, signatures, user holdings, raw chain payloads, raw provider payloads, licensed feed payloads, or application-specific crypto workflow metadata

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.finance.crypto.v1` as required but provider registration, entitlement, license, permission, credential reference, resource budget, exchange support, chain support, token support, address privacy policy, optional Web3 support, host capability, or policy admission is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, fabricate crypto prices, sign transactions, create transfer intents, strip freshness or attribution, contact a concrete provider, or fake success

#### Scenario: Optional declaration degrades explicitly
- **WHEN** an application declares `pack.finance.crypto.v1` as optional and the pack is unavailable or partially available
- **THEN** admission SHALL produce a degraded effective capability memento with unavailable commands, reason codes, provider capability hashes when safe, address-policy limitations, freshness limitations, attribution requirements, and remediation metadata
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands while still allowing discovery and diagnostics

### Requirement: Finance Crypto Pack commands SHALL use typed canonical service calls

Every `pack.finance.crypto.v1` operation SHALL be represented as a typed
`crypto.*` command/result DTO and SHALL traverse the canonical service runtime
path with trace context, policy, entitlement, license checks, address privacy
policy, resource reservation, attribution, freshness metadata, metering, health,
snapshot, structured errors, and sanitized audit behavior.

#### Scenario: Provider inspection succeeds through service runtime
- **WHEN** a declared caller invokes `crypto.inspect_provider`
- **THEN** Macaca SHALL route the typed command through SDK/facade helpers into the service runtime and crypto finance service provider
- **AND** the result SHALL include bounded provider capability, supported assets/chains/exchanges, market pairs, quote/trade/bar/snapshot support, supply support, token reference support, address-balance diagnostics support, oracle/feed support, attribution requirements, quota class, lifecycle, health, and compatibility diagnostics
- **AND** trace and audit events SHALL contain stable trace identifiers and sanitized descriptor metadata only

#### Scenario: Command is denied before provider invocation
- **WHEN** policy, permission, entitlement, license, attribution, address privacy, resource, freshness, asset, chain, exchange, pair, interval, optional Web3, or artifact checks reject a `crypto.*` command
- **THEN** Macaca SHALL return a typed denied, license-denied, address-policy-denied, quota, stale-data, web3-unavailable, unsupported, or unavailable result before invoking any concrete provider
- **AND** the audit trail SHALL include bounded reason codes without private keys, signatures, raw chain payloads, raw provider payloads, licensed feed payloads, credentials, user holdings, or unbounded crypto datasets

#### Scenario: Provider does not support a command
- **WHEN** the active provider descriptor does not support a requested command such as `crypto.inspect_public_address_balance` or `crypto.get_supply_metrics`
- **THEN** Macaca SHALL return a typed unsupported result with descriptor hash, provider capability hash, command name, and safe remediation diagnostics
- **AND** SDK discovery SHALL report the command as non-callable for the current effective capability set

### Requirement: Finance Crypto Pack SHALL expose provider-neutral DTOs and stable hashes

`pack.finance.crypto.v1` SHALL define provider-neutral DTOs and deterministic
hashing for `CryptoScope`, `CryptoProviderCapability`, `CryptoAsset`,
`TokenReference`, `ChainNetworkReference`, `CryptoExchangeVenue`,
`CryptoMarketPair`, `CryptoQuote`, `CryptoTrade`, `CryptoBar`,
`CryptoBarSeries`, `CryptoSnapshot`, `CryptoSupplyMetric`,
`PublicAddressBalanceReference`, `CryptoOracleFeedReference`,
`CryptoFreshness`, `CryptoAttribution`, `CryptoCursor`, and
`CryptoArtifactHandle`. Provider-specific extensions SHALL be bounded as
adapter metadata and SHALL NOT drive OS-layer routing.

#### Scenario: Handles and hashes remain replayable
- **WHEN** Macaca records an asset lookup, token reference, chain reference, market pair, quote, trade, bar series, snapshot, supply metric, public address diagnostic, oracle/feed reference, freshness report, cursor, artifact handle, or service snapshot
- **THEN** it SHALL include stable descriptor, capability, asset, token, chain, venue/pair, request, result, address reference, freshness, attribution, cursor, artifact, event cursor, and redaction hashes
- **AND** replay diagnostics SHALL be able to correlate the bounded evidence chain without reconstructing raw chain payloads, raw provider payloads, licensed feed payloads, private keys, signatures, or unbounded datasets

#### Scenario: Provider metadata is bounded
- **WHEN** a provider returns symbol, exchange, chain, token, contract, address, quote, trade, bar, supply, oracle, entitlement, attribution, freshness, or license metadata
- **THEN** the crypto finance service provider SHALL normalize it into provider-neutral DTO fields or bounded `adapter_metadata`
- **AND** the microkernel, SDK, shell, and generic application framework SHALL NOT branch on provider names, exchange names, chain names, token names, contract-address names, wallet names, dataset names, plan names, or application workflow names

### Requirement: Finance Crypto Pack SHALL preserve read-only crypto data boundaries

Macaca SHALL treat `pack.finance.crypto.v1` as a read-only data capability. It
SHALL NOT trade, route exchange orders, manage wallets, hold private keys, sign,
create transfer intents, create swap intents, stake, bridge, execute DeFi
actions, manage portfolio holdings, or provide investment advice.

#### Scenario: Public address diagnostic is read-only and privacy-gated
- **WHEN** a caller invokes `crypto.inspect_public_address_balance`
- **THEN** Macaca SHALL validate address privacy policy, chain scope, optional explorer/Web3 availability, entitlement, freshness policy, and redaction before returning `PublicAddressBalanceReference`
- **AND** traces, audits, snapshots, and SDK diagnostics SHALL use hashed/bounded address references and SHALL NOT expose raw private keys, seed phrases, signatures, or wallet custody data

#### Scenario: Transfer behavior is rejected
- **WHEN** a caller attempts to use the crypto pack for signing, transfer intent, swap intent, staking, bridge, order placement, or DeFi execution
- **THEN** Macaca SHALL reject the behavior as unsupported by `pack.finance.crypto.v1`
- **AND** diagnostics SHALL point to optional Web3/EVM/payment service boundaries without invoking any wallet, signing, exchange order, or chain execution provider

#### Scenario: Crypto quote preserves venue and freshness
- **WHEN** a caller invokes `crypto.get_quote`
- **THEN** Macaca SHALL return `CryptoQuote` with asset or pair handle, quote currency, venue/source timestamp, cache timestamp, freshness class, attribution, and redaction class
- **AND** it SHALL NOT omit stale, delayed, exchange-specific, or provider-specific attribution metadata required by policy

### Requirement: Finance Crypto Pack SHALL enforce permissions, entitlement, licensing, address privacy, resource, and attribution gates

Macaca SHALL gate `pack.finance.crypto.v1` with explicit permission scopes:
`crypto.provider.inspect`, `crypto.asset.search`, `crypto.asset.read`,
`crypto.token.read`, `crypto.market_pair.search`, `crypto.quote.read`,
`crypto.trade.read`, `crypto.bars.read`, `crypto.snapshot.read`,
`crypto.supply.read`, `crypto.market_status.read`,
`crypto.public_address.read`, `crypto.freshness.read`, and
`crypto.artifact.read`. Reads SHALL also pass entitlement, license, address
privacy, freshness, attribution, resource, cache, optional Web3, and output
policy checks.

#### Scenario: License or entitlement is missing
- **WHEN** a caller requests premium, exchange-licensed, chain-indexed, address-sensitive, region-restricted, redistribution-sensitive, or provider-plan-limited crypto data without the required entitlement
- **THEN** Macaca SHALL return typed `license_denied`, `address_policy_denied`, `unavailable`, or `denied` diagnostics before invoking the provider
- **AND** the audit evidence SHALL identify bounded entitlement, license, and privacy reason codes without exposing raw credentials or provider payloads

#### Scenario: Data is stale or chain-specific
- **WHEN** provider, exchange, cache, or chain timestamps indicate delayed, cached, stale, fork-risk, or finality-sensitive data
- **THEN** Macaca SHALL include `CryptoFreshness` with source timestamp, cache timestamp, chain height when applicable, finality/fork risk class, stale reason, and replay pointer
- **AND** it SHALL NOT represent stale or finality-sensitive data as confirmed real-time data

#### Scenario: Resource budget is insufficient
- **WHEN** requested result size, historical range, page count, asset count, pair count, bar count, address reference count, provider quota, chain/explorer quota, network transfer, timeout, memory, storage, or retained snapshot budget exceeds policy
- **THEN** Macaca SHALL reject the request with a typed quota/resource result
- **AND** the concrete provider SHALL NOT be invoked for rejected requests

### Requirement: Finance Crypto Pack SHALL model pagination, cache artifacts, and diagnostics explicitly

Large or paged crypto reads SHALL return explicit cursors and artifact handles
rather than unbounded payloads. Cache/artifact handles SHALL carry license,
freshness, chain/network, address privacy, retention, attribution, request hash,
and replay metadata.

#### Scenario: Historical bars are paged
- **WHEN** a `crypto.get_bars` request spans more data than the configured page limit
- **THEN** Macaca SHALL return a bounded page with `CryptoCursor`, request hash, freshness metadata, attribution metadata, and next-page diagnostics
- **AND** callers SHALL use the cursor through the canonical service path rather than provider-specific pagination APIs

#### Scenario: Artifact handle is resolved safely
- **WHEN** a caller invokes `crypto.get_artifact_handle`
- **THEN** Macaca SHALL enforce artifact permission, retention policy, entitlement, license, address privacy, attribution, and redaction before returning bounded artifact metadata
- **AND** the result SHALL NOT include raw chain payloads, raw provider payloads, licensed feed payloads, signed provider URLs beyond policy, or unbounded datasets

### Requirement: Finance Crypto Pack SHALL provide sanitized trace, audit, health, snapshot, and replay evidence

`pack.finance.crypto.v1` SHALL emit sanitized declaration, admission,
provider-inspection, asset-search, asset-read, token-reference-read,
market-pair-search, quote-read, trade-read, bars-read, snapshot-read,
supply-read, market-status, public-address-diagnostics, freshness, artifact,
policy, entitlement, license, privacy, resource, health, unavailable, failure,
snapshot, and replay events. Snapshots SHALL be bounded and replayable.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.finance.crypto.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, command availability, provider health, policy template hash, resource counters, bounded asset/chain/pair/address/request/cursor/artifact summaries, event cursors, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, private keys, seed phrases, signatures, user holdings, raw provider payloads, licensed feed payloads, raw chain payloads, manifests, package bytes, and unbounded crypto datasets

#### Scenario: Replay follows the canonical path
- **WHEN** audit replay reconstructs a `crypto.*` command chain
- **THEN** it SHALL show descriptor admission, SDK/facade service call, policy decision, entitlement/license/privacy decision, resource decision, provider dispatch, freshness/attribution evidence, cursor/artifact state, and result evidence
- **AND** replay SHALL NOT require direct provider APIs, raw chain payloads, raw provider payloads, licensed feed payloads, wallet state, private keys, signatures, or shell-owned state

### Requirement: Finance Crypto Pack SHALL preserve Macaca architecture boundaries

The `pack.finance.crypto.v1` implementation SHALL preserve Macaca's microkernel,
service runtime, application framework, SDK, runtime-host, plugin, and shell
boundaries. Concrete crypto data providers SHALL be replaceable Strategy
adapters created only by approved runtime-host composition roots. SDK helpers
SHALL only build typed service commands and SHALL NOT create providers, access
private keys, sign, create transfers, trade, call vendor APIs directly, remove
attribution, hide freshness, or infer advice.

#### Scenario: Dependency gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, canonical execution-path, and shell-boundary gates scan the implementation
- **THEN** they SHALL find no concrete CoinGecko, CoinMarketCap, Coinbase, Kraken, Binance, OKX, Etherscan, Chainlink, DefiLlama, CCXT, exchange, explorer, wallet, signing, chain client, cache, entitlement, credential-manager, or provider adapter imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed `crypto.*` service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable crypto data provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract, unavailable behavior, health semantics, trace shape, audit semantics, freshness model, address privacy model, and attribution model
- **AND** provider-specific details SHALL appear only as sanitized descriptor/capability data, not as OS-layer routing branches

### Requirement: Finance Crypto Pack SHALL include industrial developer documentation

Macaca SHALL include detailed developer documentation for
`pack.finance.crypto.v1` at `docs/developer-packs/finance/crypto.md` before
implementation completion. The documentation SHALL describe capability
declaration, required versus optional behavior, DTOs, commands, permissions,
entitlement, licensing, address privacy, freshness, attribution, read-only
boundaries, Web3/wallet/transfer non-goals, pagination, cache/artifact
handling, provider replacement, unavailable states, trace/audit/replay,
conformance tests, and supplier/API mapping.

#### Scenario: Developer reads the guide
- **WHEN** a developer opens `docs/developer-packs/finance/crypto.md`
- **THEN** the guide SHALL explain crypto assets, token references, chain/network references, exchange venues, market pairs, quotes, trades, bars, snapshots, supply metrics, public address balance references, oracle/feed references, freshness, attribution, entitlements, licenses, cursors, artifacts, diagnostics, and operational limits
- **AND** examples SHALL use synthetic assets, chains, pairs, addresses, quotes, bars, metrics, and artifacts only

#### Scenario: Provider author checks conformance
- **WHEN** a provider author uses the documentation to implement a provider
- **THEN** the guide SHALL include conformance checks for descriptor completeness, DTO compatibility, command support, stable hashing, scope validation, asset/token/chain/exchange/pair support, quote/trade/bar validation, supply metric validation, public address privacy enforcement, freshness labeling, attribution enforcement, license checks, pagination, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and redaction
- **AND** the guide SHALL map CoinGecko, CoinMarketCap, Coinbase, Kraken, Binance, OKX, Etherscan-like explorers, Chainlink, DefiLlama, CCXT, exchange feeds, explorer APIs, optional Web3 handoff, cache, entitlement, and attribution concepts to Macaca abstractions without making supplier-specific behavior OS semantics
