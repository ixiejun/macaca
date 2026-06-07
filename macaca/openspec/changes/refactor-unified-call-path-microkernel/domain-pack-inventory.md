# Domain Pack Inventory — P4 §5.2.1

> Impact memo (GitNexus non-blocking): finance extraction touches `bootstrap_domain_pack_services`,
> `service_runtime_wiring`, `InMemoryDomainPackCatalog`, and optional `macaca-domain-pack-finance` crate.

## Current state (iteration 38 baseline)

| Location | Role | Production reachable? | Action |
|----------|------|---------------------|--------|
| `runtime-host/domain_pack_service_provider.rs` | Generic `DomainPackProviderRegistration` + `bootstrap_domain_pack_services` | Yes | **Keep** (generic only) |
| `runtime-host/domain_pack_service_provider.rs::finance_fixture` | Test constants + `extract_symbol` | Tests only | **Move** to finance crate |
| `runtime-host/finance_live_data.rs` | Binance/OKX/Coindesk adapters | `#[cfg(test)]` only | **Move** to finance crate |
| `runtime-host/finance_llm_analysis_provider.rs` | LLM analysis provider | Exported but not bootstrapped | **Move** to finance crate |
| `runtime-host/lib.rs` | `pub use FinanceLlmAnalysisSystemServiceProvider` | Dead export | **Remove** |
| `web/service_runtime_wiring.rs` | `bootstrap_builtin_domain_pack_services` | Yes (returns empty) | **Replace** with `bootstrap_domain_pack_services([])` |
| `macaca-app/service_capability.rs` | `with_builtin_defaults()` registers `pack.finance.v1` | Yes | **Empty default**; finance catalog in package crate |
| `macaca-app` tests / loaders | `pack.finance.v1` in fixtures | Tests only | **Use** finance crate catalog helper |

## Target architecture

```
Composition root (web / custom host)
  └─ optional: macaca-domain-pack-finance::finance_domain_pack_registrations(llm)
       └─ bootstrap_domain_pack_services(runtime, registrations, trace_prefix)

Base runtime-host
  └─ domain_pack_service_provider (generic registration only, no domain strings)

Optional package
  └─ macaca-domain-pack-finance (Bridge + Strategy + Abstract Factory)
       ├─ contract (service ids, descriptors, symbol extraction)
       ├─ live_data (exchange/RSS adapters)
       ├─ data_provider (market/financials/news)
       ├─ llm_analysis_provider
       └─ bootstrap (registrations + catalog definition)
```

## Service surface (`pack.finance.v1`)

| Service id | Provider | Commands |
|------------|----------|----------|
| `service.market_data` | `FinanceDataSystemServiceProvider` | `finance.lookup` |
| `service.financials` | `FinanceDataSystemServiceProvider` | `finance.lookup` |
| `service.news_digest` | `FinanceDataSystemServiceProvider` | `finance.lookup` |
| `service.llm.analysis` | `FinanceLlmAnalysisSystemServiceProvider` | `finance.analyze` |

## Absent-pack behavior

- Base OS bootstrap registers **zero** domain-pack providers.
- `service.call` to finance service ids returns structured **unavailable** (not synthetic success).
- Catalog resolution without installed pack metadata yields `unresolved_packs` audit evidence.

## VC-hardcoded gate (§5.3.2)

Production `macaca-runtime-host/src/**/*.rs` must not contain:
`Binance`, `OKX`, `Coindesk`, `coindesk`, `pack.finance`, `asset_class.*crypto`, `finance.lookup`, `finance.analyze`.

Fixtures/tests in other crates are exempt.
