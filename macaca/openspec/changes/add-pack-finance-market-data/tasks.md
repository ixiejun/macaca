## 1. Supplier API Research And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, OpenSpec rules, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Study Polygon/Massive market data APIs for ticker reference, latest quotes, latest trades, aggregates/bars, snapshots, market status, corporate actions, pagination, entitlements, and errors.
- [x] 1.3 Study Alpaca Market Data APIs for real-time and historical equities/options/crypto bars, quotes, trades, snapshots, subscription tiers, rate limits, and asset-class boundaries.
- [x] 1.4 Study Nasdaq Data Link APIs for real-time, delayed, table, time-series, dataset, streaming, attribution, and licensing semantics.
- [x] 1.5 Study Finnhub APIs for quote, candles, symbol lookup, market status, fundamentals/economic dataset boundaries, quotas, and errors.
- [x] 1.6 Study Alpha Vantage, Tiingo, Intrinio, and exchange direct-feed documentation as baselines for adjusted bars, corporate actions, identifiers, reference datasets, entitlements, exchange licensing, and provider attribution.
- [x] 1.7 Produce a supplier capability comparison memo mapping vendor/exchange/cached/historical data concepts into Macaca provider-neutral market data DTOs and commands.
- [x] 1.8 Define explicit non-goals for trading, order routing, investment advice, portfolio allocation, brokerage workflow, raw provider pass-through, licensed feed observability, and provider/exchange/symbol/dataset-specific routing.
- [x] 1.9 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.finance.market.data.v1` descriptor metadata: pack id, family, lifecycle, stability, asset classes, venues, real-time/delayed/end-of-day support, quote/trade/bar/snapshot/corporate-action support, market-status support, freshness diagnostics, interval support, adjustment support, identifiers, pagination model, attribution requirements, auth modes, rate limits, command schemas, permission scopes, policy templates, resource budgets, approval requirements, data-governance class, SDK metadata, documentation link, compatibility, and diagnostics.
- [x] 2.2 Define provider-neutral DTOs for `MarketDataScope`, `MarketDataProviderCapability`, `InstrumentHandle`, `InstrumentIdentity`, `MarketVenue`, `MarketSession`, `MarketQuote`, `MarketTrade`, `MarketBar`, `MarketBarSeries`, `MarketSnapshot`, `CorporateAction`, `MarketDataFreshness`, `MarketDataAttribution`, `MarketDataCursor`, and `MarketDataArtifactHandle`.
- [x] 2.3 Define typed command/result DTOs for `market_data.inspect_provider`, `market_data.search_instruments`, `market_data.get_instrument`, `market_data.get_quote`, `market_data.get_trade`, `market_data.get_bars`, `market_data.get_snapshot`, `market_data.get_corporate_actions`, `market_data.inspect_market_status`, `market_data.inspect_freshness`, and `market_data.get_artifact_handle`.
- [x] 2.4 Define typed success, paged, partial, denied, unavailable, unsupported, conflict, stale-data, schema-mismatch, provider-attribution-required, license-denied, symbol-ambiguous, symbol-not-found, exchange-unsupported, asset-class-unsupported, range-too-large, interval-unsupported, adjustment-unsupported, quota, timeout, cancellation, and failure DTOs.
- [x] 2.5 Define stable descriptor hashing, provider capability hashing, instrument identity hashing, venue/session hashing, request hashing, quote/trade/bar/snapshot hashing, corporate-action hashing, freshness hashing, attribution hashing, cursor hashing, artifact handle hashing, event cursor hashing, and redaction metadata.
- [x] 2.6 Add descriptor and DTO compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, schema evolution, instruments, venues, sessions, quotes, trades, bars, snapshots, corporate actions, freshness, attribution, cursors, artifacts, and serde compatibility.

## 3. Admission, Permission, Policy, Resource, Entitlement, And Licensing

- [x] 3.1 Implement manifest declaration validation for required and optional `pack.finance.market.data.v1` declarations.
- [x] 3.2 Implement permission validation for `market_data.provider.inspect`, `market_data.instrument.search`, `market_data.instrument.read`, `market_data.quote.read`, `market_data.trade.read`, `market_data.bars.read`, `market_data.snapshot.read`, `market_data.corporate_actions.read`, `market_data.market_status.read`, `market_data.freshness.read`, and `market_data.artifact.read`.
- [ ] 3.3 Implement provider/instrument/venue/dataset/cursor/artifact scope checks for declared providers, asset classes, venues, datasets, real-time data, premium data, denied scopes, stale handles, and redistribution-sensitive data.
- [ ] 3.4 Implement policy checks for symbol search bounds, identifier resolution, asset class support, exchange/venue support, date range, interval, pagination, freshness policy, adjustment policy, currency/timezone metadata, attribution requirements, cache policy, and output redaction.
- [ ] 3.5 Implement resource reservation for result size, historical range, page count, instrument count, bar count, corporate action count, provider quota, network transfer, timeout, memory, storage, streaming output if supported, and retained snapshots.
- [ ] 3.6 Implement entitlement and license checks with structured unavailable/denied diagnostics for missing provider, disabled pack, missing credential reference, missing permission, missing real-time entitlement, missing exchange license, redistribution denial, unsupported venue, unsupported asset class, provider quota, stale cache, and host resource denial.
- [ ] 3.7 Implement approval behavior for redistribution-sensitive, real-time, exchange-licensed, premium, region-restricted, very large historical, or external-publication requests.
- [ ] 3.8 Add tests proving denied, validation, quota, unavailable, conflict, stale-data, schema-mismatch, attribution-required, license-denied, symbol-ambiguous, symbol-not-found, exchange-unsupported, asset-class-unsupported, range-too-large, interval-unsupported, adjustment-unsupported, timeout, and cancellation paths do not call concrete providers, leak licensed payloads, produce fake prices, strip attribution, or expose unbounded datasets.

## 4. Service Provider And Runtime Integration

- [ ] 4.1 Implement or bind the market data service provider behind the service runtime; do not construct market data providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add a deterministic unavailable provider that returns typed unavailable/unsupported diagnostics and complete discovery metadata.
- [ ] 4.3 Add mock provider support for provider inspection, instrument search, instrument metadata, quotes, trades, bars, snapshots, corporate actions, market status, freshness inspection, artifact handles, health, and provider capability inspection.
- [ ] 4.4 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, bounded paging, cache/artifact behavior, stale-data diagnostics, provider attribution diagnostics, schema/symbol/venue mismatch diagnostics, license diagnostics, and rate-limit diagnostics.
- [ ] 4.5 Add Strategy implementations for provider adapters, symbol resolvers, identifier mappers, quote readers, trade readers, bar readers, snapshot readers, corporate-action readers, market-status readers, freshness resolvers, attribution resolvers, cache readers, redaction, and unavailable behavior.
- [ ] 4.6 Add side-effect safety support for cache-producing reads, request hashes, provider state validation, entitlement state, license state, attribution state, freshness state, cursor validation, artifact retention, and bounded replay.
- [ ] 4.7 Add provider capability reporting for available, degraded, preview, unavailable, unsupported, retired, asset-class-limited, venue-limited, delayed-only, end-of-day-only, quote-limited, history-limited, corporate-action-limited, license-limited, attribution-required, network-limited, and quota-limited states.

## 5. SDK, WASM ABI, Application Framework, And Examples

- [x] 5.1 Extend SDK discovery for `pack.finance.market.data.v1` with command schemas, asset classes, venue support, quote/trade/bar/snapshot/corporate-action support, interval support, adjustment support, identifier support, freshness classes, attribution requirements, examples, availability, diagnostics, documentation link, provider class, capability hash, compatibility, and redaction profile.
- [x] 5.2 Extend application admission so required declarations block readiness when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders for all `market_data.*` commands; helpers must only build canonical traced service calls and must never construct market data clients, access credentials, query provider APIs directly, remove attribution, hide freshness, infer advice, trade, or bypass policy.
- [ ] 5.4 Extend WASM/app ABI descriptors so applications can discover market data commands, declare permissions, receive unavailable diagnostics, and submit typed service calls through the canonical execution path.
- [x] 5.5 Add generic app-facing examples for provider inspection, instrument search, instrument metadata, quote, trade, bars, snapshot, corporate actions, market status, freshness, artifact handles, and unavailable diagnostics.
- [x] 5.6 Add unavailable-provider, missing-entitlement, license-denied, stale-data, delayed-only, symbol-ambiguous, symbol-not-found, exchange-unsupported, asset-class-unsupported, range-too-large, interval-unsupported, adjustment-unsupported, attribution-required, provider-quota, network-denied, and artifact-denied examples using synthetic data only.

## 6. Trace, Audit, Replay, Security, And Gates

- [ ] 6.1 Emit sanitized declaration, admission, provider-inspection, instrument-search, instrument-read, quote-read, trade-read, bars-read, snapshot-read, corporate-action-read, market-status, freshness, artifact-handle, policy, entitlement, license, resource, health, snapshot, unavailable, and failure events.
- [ ] 6.2 Ensure traces, audits, snapshots, SDK diagnostics, and examples exclude raw credentials, account identifiers, user holdings, raw provider payloads, licensed feed payloads, manifests, package bytes, private keys, signatures, and unbounded market datasets.
- [ ] 6.3 Add replay tests proving every `market_data.*` command is trace-addressable through the canonical service path and that snapshots contain enough bounded metadata for recovery diagnostics.
- [ ] 6.4 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete Polygon/Massive, Alpaca, Nasdaq, Finnhub, Alpha Vantage, Tiingo, Intrinio, exchange-feed, cache, entitlement, credential-manager, or provider adapters.
- [ ] 6.5 Add no-direct-provider-call gates proving SDK helpers, WASM ABI handlers, app admission, web, CLI, and frontend paths route through descriptor-owned service commands.
- [ ] 6.6 Add boundary tests proving optional provider absence returns structured unavailable diagnostics and never crashes, hangs, silently falls back, fabricates market prices, calls providers, strips freshness, strips attribution, leaks licensed payloads, or fakes success.
- [ ] 6.7 Run `openspec validate add-pack-finance-market-data --strict`, targeted cargo tests, boundary gates, file-size gates, and audit replay checks before marking implementation complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/finance/market-data.md` with purpose, capability model, manifest declaration, required versus optional behavior, permissions, provider scopes, instruments, identifiers, venues, sessions, quotes, trades, bars, snapshots, corporate actions, freshness, attribution, entitlements, licenses, cursors, artifacts, unavailable diagnostics, provider replacement, and operational limits.
- [x] 7.2 Document all command DTOs and result DTOs with field-level explanations, pagination semantics, cache semantics, redaction behavior, timeout/cancellation behavior, attribution behavior, license behavior, freshness classes, stale-data diagnostics, adjustment policies, currency/timezone handling, interval/range limits, artifact retention, and structured error codes.
- [x] 7.3 Document supplier/API mapping: Polygon/Massive, Alpaca, Nasdaq Data Link, Finnhub, Alpha Vantage, Tiingo, Intrinio, exchange direct-feed, cache, entitlement, and attribution concepts mapped to Macaca abstractions, including what is intentionally not exposed as OS semantics.
- [x] 7.4 Add generic examples for provider inspection, instrument search, instrument metadata, quote, trade, bars, snapshot, corporate action, market status, freshness, artifact handles, stale diagnostics, and unavailable diagnostics using synthetic market data only.
- [x] 7.5 Add conformance checklist and test guidance for provider authors: descriptor completeness, instrument/venue/dataset scope validation, asset-class support, quote/trade/bar validation, corporate-action validation, freshness labeling, attribution enforcement, license checks, pagination, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and redaction.
- [x] 7.6 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-finance-market-data` complete.
