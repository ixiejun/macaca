# Finance Crypto Pack

`pack.finance.crypto.v1` describes provider-neutral, read-only crypto reference
and market data capabilities. The descriptor is discoverable through SDK
catalogs, but commands remain unavailable until a crypto data provider is
installed through the runtime composition root.

## Manifest Declaration

Declare the pack as required only when crypto reference data is mandatory for
readiness. Optional declarations degrade with structured unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.finance.crypto.v1"]
```

## Permissions

Use the narrowest scope: `crypto.provider.inspect`, `crypto.asset.search`,
`crypto.asset.read`, `crypto.token.read`, `crypto.market_pair.search`,
`crypto.quote.read`, `crypto.trade.read`, `crypto.bars.read`,
`crypto.snapshot.read`, `crypto.supply.read`, `crypto.market_status.read`,
`crypto.public_address.read`, `crypto.freshness.read`, and
`crypto.artifact.read`.

## Capability Model

Macaca models crypto data as tenant and chain scopes, provider capability
reports, crypto assets, token references, chain/network references, exchange
venues, market pairs, quotes, trades, bar series, snapshots, supply metrics,
public address balance references, oracle feed references, freshness records,
attribution records, cursors, and artifact handles. The model carries identity
hashes, chain height, finality class, address privacy profile, license class,
freshness class, and bounded pagination metadata. Private keys, seed phrases,
signatures, wallet custody, transfers, swaps, orders, holdings, raw chain
payloads, raw provider payloads, licensed feed payloads, and unbounded datasets
stay behind provider adapters or other service boundaries.

## Commands And Results

`crypto.inspect_provider`, `crypto.search_assets`, `crypto.get_asset`,
`crypto.get_token_reference`, `crypto.search_market_pairs`,
`crypto.get_quote`, `crypto.get_trade`, `crypto.get_bars`,
`crypto.get_snapshot`, `crypto.get_supply_metrics`,
`crypto.inspect_market_status`, `crypto.inspect_public_address_balance`,
`crypto.inspect_freshness`, and `crypto.get_artifact_handle` are
descriptor-owned schema names.

Every command uses a `FinanceCommandEnvelope` with `subject_ref`,
string parameters, optional cursor, optional page size, and optional
idempotency key. Results use `CryptoResultEnvelope<T>` and may carry a single
DTO, a bounded `FinancePage<T>`, or a sanitized `FinanceError`. Status values
include success, paged, partial, denied, unavailable, unsupported, conflict,
stale-data, schema-mismatch, provider-attribution-required, license-denied,
asset-ambiguous, asset-not-found, chain-unsupported, exchange-unsupported,
pair-unsupported, interval-unsupported, address-policy-denied,
web3-unavailable, range-too-large, quota, timeout, cancellation, and failure.

Token references carry chain id, contract address hash, and decimals. Quotes,
trades, and bars carry pair ids, timestamps, freshness, and attribution.
Snapshots carry hashes to latest quote, trade, and supply objects instead of
embedding unbounded exchange or chain payloads. Public address balance
diagnostics return privacy-gated references, never raw wallet ownership claims.
Oracle feed references describe feed identity and freshness without making
oracle-specific protocols OS semantics.

## Supplier Mapping

CoinGecko coins, metadata, market data, historical charts, exchanges, and
categories map to assets, metadata, quotes, bars, venues, and provider
capability. CoinMarketCap listings, quotes, metadata, categories, exchange
data, market pairs, credits, and attribution map to assets, quotes, pairs,
quota diagnostics, and attribution. Coinbase, Kraken, Binance, and OKX product
lists, tickers, candles, trades, and status map to venues, pairs, quotes, bars,
trades, sessions, and freshness. Etherscan-like explorers, Chainlink feeds, and
DefiLlama datasets map to token references, public address references, oracle
feed references, and supply metrics. CCXT informs capability discovery and
symbol normalization only. Provider-specific endpoints, chain RPC payloads,
wallet objects, signing flows, order flows, subscription names, and routing
rules are not OS semantics.

## App-Facing Examples

- Inspect provider classes and unavailable diagnostics before crypto reads.
- Search assets, resolve token references, inspect chain/network references,
  and search market pairs through bounded requests.
- Read quotes, trades, bars, snapshots, supply metrics, market status,
  public-address diagnostics, freshness, and artifact handles by reference.
- Treat address diagnostics as privacy-gated references and never as wallet
  ownership proof.
- Treat missing provider, missing entitlement, license-denied, stale data,
  asset-ambiguous, asset-not-found, chain-unsupported, exchange-unsupported,
  pair-unsupported, interval-unsupported, address-policy-denied,
  web3-unavailable, attribution-required, provider-quota, network-denied,
  timeout, and artifact-denied states as structured results. Synthetic
  examples must use synthetic assets, pairs, addresses, and prices only.

## Trace And Audit

Traces should record declaration, admission decision, command name, asset id,
token reference hash, chain id, venue id, pair id, request hash, address hash,
provider class, capability hash, freshness class, finality class, attribution
reference, result status, cursor hash, artifact id, and redaction profile. They
must not record credentials, private keys, seed phrases, signatures, wallet
holdings, raw provider payloads, raw chain payloads, licensed feed payloads,
manifests, package bytes, or unbounded crypto datasets.

## Provider Authors

Conformance requires descriptor completeness, asset, token, chain, exchange,
pair, address, cursor, and artifact scope validation, quote validation, trade
validation, bar range and interval validation, supply metric validation, public
address privacy enforcement, oracle feed validation, freshness labeling,
attribution enforcement, license checks, pagination, resource bounds, timeout
and cancellation handling, policy hooks, trace and audit events, unavailable
behavior, snapshot and replay metadata, and redaction tests. Providers must
return structured unavailable, denied, unsupported, conflict, stale-data,
schema-mismatch, license-denied, address-policy-denied, web3-unavailable,
range-too-large, quota, timeout, cancellation, and failure results without
signing transactions, creating transfers, placing orders, or fabricating prices.
