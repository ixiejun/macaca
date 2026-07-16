# Finance Crypto Pack Design

## Context

`pack.finance.crypto.v1` exposes crypto-asset reference and market data as a
Macaca OS serviceized capability. It lets applications discover crypto assets,
inspect token references, resolve exchange market pairs, retrieve quotes,
trades, bars, snapshots, supply metrics, market status, and optional read-only
public address balance diagnostics without embedding exchange APIs, aggregator
APIs, explorer APIs, oracle clients, wallet providers, or application-specific
crypto workflows into generic OS layers.

The pack is read-only. Wallet custody, private keys, signing, transfer intents,
swap intents, staking, bridging, DeFi execution, and chain transactions belong
to optional Web3/EVM/payment services, not this finance data pack.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| CoinGecko | Coins, metadata, market data, historical charts, exchanges, categories | Crypto asset, metadata, quote, bars, exchange, freshness |
| CoinMarketCap | Listings, quotes, metadata, categories, exchanges, market pairs | Asset listing, market pair, quote, attribution, entitlement |
| Coinbase / Kraken / Binance / OKX | Exchange products/pairs, ticker, candles, trades, order-book summaries, exchange status | Exchange venue, market pair, quote/trade/bar, market status |
| Etherscan-like explorers | Read-only token metadata, balances, transactions, contract metadata | Token reference, public address balance reference, chain/network scope |
| Chainlink / DefiLlama | Oracle/feed metadata, DeFi reference datasets, TVL/supply style data | Oracle/feed reference, supply metric, dataset attribution |

The pack exposes provider-neutral contracts. Provider adapters translate to
aggregators, exchanges, explorers, oracle feeds, cache stores, entitlement
systems, or unavailable providers. OS layers must not branch on provider names,
exchange names, chains, tokens, contract addresses, wallet addresses, dataset
names, or business workflows.

## Goals

- Provide stable pack id `pack.finance.crypto.v1` and command namespace
  `crypto.*`.
- Support provider inspection, crypto asset search, asset metadata, token
  reference lookup, chain/network reference lookup, exchange market pair
  discovery, quote/trade/bar/snapshot retrieval, supply metric retrieval,
  market status, optional public address balance diagnostics, oracle/feed
  references, freshness, attribution, entitlement diagnostics, artifacts,
  health, snapshots, and replay diagnostics.
- Preserve financial and privacy safety with read-only semantics, address
  privacy policy, chain/network scope, exchange/provider attribution, freshness
  labels, entitlement checks, pagination, bounded output, and sanitized audit.
- Keep concrete crypto data providers behind replaceable service providers.
- Require developer documentation at `docs/developer-packs/finance/crypto.md`.

## Non-Goals

- Do not implement concrete CoinGecko, CoinMarketCap, Coinbase, Kraken,
  Binance, OKX, Etherscan, Chainlink, DefiLlama, CCXT, exchange, explorer,
  oracle, wallet, signing, cache, or entitlement providers in this proposal.
- Do not define trading, exchange order routing, wallet custody, private key
  management, signing, transfer intent, swap intent, staking, bridge, DeFi
  execution, tax advice, portfolio holdings, investment advice, or
  application-specific crypto workflows.
- Do not expose raw credentials, private keys, seed phrases, signatures, user
  holdings, raw provider payloads, licensed feed payloads, raw chain payloads,
  manifests, package bytes, or unbounded crypto datasets in observability.
- Do not silently substitute providers, infer investment advice, remove
  attribution, hide stale data, sign transactions, create transfer intents, or
  fake success when provider, exchange, chain, token, entitlement, license,
  address policy, freshness, permission, resource, Web3, or host support is
  absent.

## Ownership And Boundaries

- Pack id: `pack.finance.crypto.v1`.
- Family: `finance`.
- Backing service owner: crypto finance service provider.
- SDK surface: `sdk.packs.finance.crypto`.
- Command namespace: `crypto.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, credential bridges, optional
  Web3/market-data handoff bridges, entitlement/cache bridges, decorators, and
  sanitized diagnostics through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `crypto.inspect_provider` | Inspect provider, asset, exchange, chain, token, quote, bars, supply, address-balance, oracle/feed, and entitlement support | Returns sanitized capability, quota, lifecycle, attribution, health, and compatibility metadata |
| `crypto.search_assets` | Search crypto assets by symbol, name, identifier, chain, category, or provider listing | Requires bounded query, license policy, paging, and redaction |
| `crypto.get_asset` | Resolve crypto asset metadata | Requires asset disambiguation, chain/network references, attribution, and freshness |
| `crypto.get_token_reference` | Resolve token/contract reference metadata | Requires chain scope, contract reference policy, token metadata support, and redaction |
| `crypto.search_market_pairs` | Discover exchange or aggregator market pairs | Requires exchange/venue scope, base/quote asset scope, entitlement, and paging |
| `crypto.get_quote` | Retrieve spot quote for an asset or pair | Requires freshness class, quote currency, venue/pair policy, entitlement, and attribution |
| `crypto.get_trade` | Retrieve latest or time-scoped trade | Requires pair/venue scope, freshness class, entitlement, and bounded result |
| `crypto.get_bars` | Retrieve historical crypto bars/candles | Requires range validation, interval support, quote currency, pagination, quota, and attribution |
| `crypto.get_snapshot` | Retrieve crypto asset or pair snapshot | Requires quote/trade/bar/supply support, entitlement, stale-data metadata, and bounded result |
| `crypto.get_supply_metrics` | Retrieve circulating/total/max supply or comparable metrics | Requires source policy, timestamp, attribution, and freshness |
| `crypto.inspect_market_status` | Inspect exchange/venue or provider market status | Requires venue/provider scope and bounded market status metadata |
| `crypto.inspect_public_address_balance` | Inspect read-only public address balance metadata where permitted | Requires address privacy policy, chain scope, optional Web3/explorer availability, and redaction |
| `crypto.inspect_freshness` | Inspect source timestamp, cache timestamp, chain height when applicable, and staleness diagnostics | Requires provider/asset/pair scope and attribution |
| `crypto.get_artifact_handle` | Resolve cached/paged result artifact metadata | Requires artifact permission, retention, and licensing policy |

Every command must define typed command DTOs, typed success results, typed
paged/partial results, typed denied/unavailable/unsupported/conflict/stale-data/
schema-mismatch/provider-attribution-required/license-denied/asset-ambiguous/
asset-not-found/chain-unsupported/exchange-unsupported/pair-unsupported/
interval-unsupported/address-policy-denied/web3-unavailable/range-too-large/
quota/timeout/cancellation/failure results, redaction profile, cache semantics,
idempotency semantics for cache-producing reads, and replay metadata.

## DTO Model

Core DTOs:

- `CryptoScope`: provider scope, asset handle, chain/network scope,
  exchange/venue scope, market pair handle, public address reference when
  permitted, credential reference, entitlement state, license policy, address
  privacy policy, freshness policy, permission state, rate-limit profile, and
  health.
- `CryptoProviderCapability`: provider class, supported asset classes, chains,
  exchanges, market pairs, quote/trade/bar/snapshot support, supply metric
  support, token reference support, address balance diagnostics support,
  oracle/feed support, pagination model, attribution requirements, auth modes,
  rate limits, lifecycle, and health.
- `CryptoAsset`: asset handle, canonical symbol, name projection, asset type,
  chain references, token references, category classes, quote currencies,
  status, entitlement class, attribution class, freshness, and redaction class.
- `TokenReference`: token handle, chain/network, contract address hash,
  decimals class, standard class, verification class, issuer/project handle,
  metadata source, attribution, and redaction class.
- `ChainNetworkReference`: chain handle, network class, chain id class,
  native asset handle, finality/fork risk class, explorer/oracle availability,
  and redaction class.
- `CryptoExchangeVenue` and `CryptoMarketPair`: venue handle, pair handle,
  base/quote assets, symbol projection, precision class, min-size class, status,
  fee/venue metadata class, attribution, and freshness.
- `CryptoQuote`, `CryptoTrade`, `CryptoBar`, `CryptoBarSeries`, and
  `CryptoSnapshot`: asset/pair handles, price/size/volume/liquidity classes,
  quote currency, venue/source timestamp, cache timestamp, chain height when
  applicable, freshness class, attribution, and redaction class.
- `CryptoSupplyMetric`: asset handle, metric type, value class, timestamp,
  source class, method class, freshness, attribution, and redaction class.
- `PublicAddressBalanceReference`: chain handle, address hash, asset/token
  handle, balance class, block height class, timestamp, privacy class,
  attribution, and redaction class.
- `CryptoOracleFeedReference`: feed handle, asset/pair scope, oracle/source
  class, heartbeat/deviation class, timestamp, freshness, attribution, and
  redaction class.
- `CryptoFreshness`, `CryptoAttribution`, `CryptoCursor`, and
  `CryptoArtifactHandle`: freshness, attribution, paging/cache/artifact,
  license, checksum, redaction, and replay metadata.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Licensing Model

Permission scopes:

- `crypto.provider.inspect`
- `crypto.asset.search`
- `crypto.asset.read`
- `crypto.token.read`
- `crypto.market_pair.search`
- `crypto.quote.read`
- `crypto.trade.read`
- `crypto.bars.read`
- `crypto.snapshot.read`
- `crypto.supply.read`
- `crypto.market_status.read`
- `crypto.public_address.read`
- `crypto.freshness.read`
- `crypto.artifact.read`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, asset handle when applicable, chain/network scope
  when applicable, market pair handle when applicable, public address reference
  when applicable, credential reference, entitlement state, license policy,
  address privacy policy, freshness policy, and permission state.
- Results must preserve freshness, source timestamp, cache timestamp, chain
  height/finality when applicable, exchange/provider attribution, quote
  currency, and data-license metadata.
- Public address diagnostics require explicit address privacy policy and must
  use hashed/bounded address references in observability.
- No command may sign, transfer, swap, stake, bridge, trade, or create transfer
  intent. Such behavior must use optional Web3/EVM/payment service boundaries.
- Redistribution-sensitive, premium, exchange-licensed, chain-indexed,
  address-sensitive, or large historical requests may require entitlement or
  approval.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
supported assets, chains, exchanges, market pairs, quote/trade/bar/snapshot
support, supply support, token reference support, address-balance diagnostics,
oracle/feed support, freshness classes, attribution requirements, permission
scopes, policy templates, resource limits, approval rules, provider capability
hashes, health, compatibility, diagnostics, examples, redaction profiles, and
documentation links.

The developer guide at `docs/developer-packs/finance/crypto.md` must cover:

- manifest declaration and optional/required behavior
- crypto assets, token references, chain/network references, exchange venues,
  market pairs, quotes, trades, bars, snapshots, supply metrics, public address
  balance references, oracle/feed references, freshness, attribution,
  entitlements, licenses, cursors, artifacts, provider capabilities, and
  unavailable states
- read-only boundaries, Web3/wallet/transfer non-goals, address privacy,
  stale-data diagnostics, quote currency, exchange versus aggregator data,
  provider replacement, trace/audit interpretation, and conformance tests

Examples must use synthetic assets, chains, pairs, addresses, quotes, bars,
metrics, and artifacts. They must not include provider names, credentials,
private keys, seed phrases, signatures, real holdings, live trading strategies,
investment advice, raw chain payloads, raw provider payloads, or workflow-
specific conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `crypto_pack_declared`
- `crypto_pack_admission_validated`
- `crypto_provider_inspected`
- `crypto_assets_searched`
- `crypto_asset_resolved`
- `crypto_token_reference_read`
- `crypto_market_pairs_searched`
- `crypto_quote_read`
- `crypto_trade_read`
- `crypto_bars_read`
- `crypto_snapshot_read`
- `crypto_supply_metrics_read`
- `crypto_market_status_inspected`
- `crypto_public_address_balance_inspected`
- `crypto_freshness_inspected`
- `crypto_artifact_handle_resolved`
- `crypto_pack_policy_decision`
- `crypto_pack_service_call_requested`
- `crypto_pack_service_call_succeeded`
- `crypto_pack_service_call_failed`
- `crypto_pack_unavailable`
- `crypto_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, command
availability, provider health, policy template hash, resource counters, bounded
asset/chain/pair/address/request/cursor/artifact summaries, event cursors, and
sanitized replay pointers. Snapshots must exclude raw credentials, private keys,
seed phrases, signatures, user holdings, raw provider payloads, licensed feed
payloads, raw chain payloads, manifests, package bytes, and unbounded crypto
datasets.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, exchange readers, aggregator readers,
  explorer readers, oracle readers, token metadata readers, cache readers,
  attribution resolvers, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, metering, licensing,
  attribution, freshness, address privacy, cache, and output redaction wrap
  service calls.
- **Specification**: admission validates provider scope, command availability,
  permissions, entitlement, asset, chain, exchange, pair, address policy,
  range, interval, freshness policy, Web3 handoff, and compatibility.
- **Observer**: provider health, trace, audit, service events, and cache/artifact
  lifecycle events are subscribable.
- **Memento**: capability hashes, request hashes, cursors, cache handles,
  snapshots, and replay pointers preserve recovery state.
- **Abstract Factory**: concrete crypto data providers are created only by
  approved runtime-host composition roots.

## Risks And Mitigations

- Risk: pack drifts into wallet or trading semantics. Mitigation: read-only
  command surface and explicit non-goals for signing, transfer, swap, staking,
  bridge, and order routing.
- Risk: public address lookup leaks sensitive behavioral data. Mitigation:
  address privacy policy, hashed address references, approval/entitlement, and
  observability exclusions.
- Risk: stale or exchange-specific prices are misrepresented. Mitigation:
  freshness, venue, quote currency, source timestamp, and attribution metadata.
- Risk: raw chain or provider payloads leak. Mitigation: bounded DTOs,
  redaction, artifact boundaries, and strict observability exclusions.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call provider APIs directly.
