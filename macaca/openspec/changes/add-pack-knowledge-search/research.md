# Knowledge Search Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
existing platform inventory, and GitNexus memo evidence for
`pack.knowledge.search.v1`. Search must expose corpus, index, query, filter,
facet, sort, ranking, cursor, hit, snippet, source attribution, ACL, refresh,
stats, and explanation behavior through serviceized commands, not provider
query strings or shell-owned search semantics.

## Source Baseline

- Elasticsearch Query DSL:
  <https://www.elastic.co/docs/explore-analyze/query-filter/languages/querydsl>
- OpenSearch Query DSL:
  <https://docs.opensearch.org/latest/query-dsl/>
- Algolia Search API:
  <https://www.algolia.com/doc/rest-api/search>
- Azure AI Search:
  <https://learn.microsoft.com/en-us/azure/search/search-what-is-azure-search>
- Microsoft Graph Search API:
  <https://learn.microsoft.com/en-us/graph/api/resources/search-api-overview>
- Google Programmable Search:
  <https://developers.google.com/custom-search/v1/overview>

## Supplier API Notes

- Elasticsearch/OpenSearch contribute structured query DSLs, analyzers, filters,
  aggregations/facets, sort, pagination, highlights, explanation, index refresh,
  and stats. Macaca should model a provider-neutral query AST and capability
  report rather than passing raw DSL through SDK.
- Algolia contributes searchable attributes, filters, facets, custom ranking,
  synonyms, typo tolerance, secured keys, suggestions, replicas, and analytics.
  Macaca should expose ranking profiles, synonym/analyzer metadata, facet DTOs,
  suggest/autocomplete commands, and policy-scoped credential references.
- Azure AI Search contributes indexes, indexers, analyzers, scoring profiles,
  semantic ranking, vector/hybrid support, suggest/autocomplete, and statistics.
  Macaca should expose semantic/hybrid capability metadata without hardcoding
  Azure scoring profiles.
- Microsoft Graph Search contributes federated verticals, result types, query
  constraints, and ACL-aware result sets. Macaca should model corpus verticals,
  source attribution, and ACL trimming.
- Google Programmable Search contributes programmable engines, refinements,
  result attribution, and provider query limits. Macaca should map these to
  corpus handles, refinement/filter capability, attribution, and quota metadata.

## Macaca-Owned Abstractions

`pack.knowledge.search.v1` should define `SearchCorpus`,
`SearchIndexSchema`, `SearchField`, `SearchAnalyzerProfile`,
`SearchSynonymSet`, `SearchRankingProfile`, `SearchQuery`, `SearchFilter`,
`SearchFacetRequest`, `SearchSort`, `SearchHit`, `SearchCursor`,
`SearchRankingExplanation`, and `SearchProviderCapability`.

The DTOs must carry declared fields, analyzer and ranking metadata, query AST,
facet/sort constraints, snippet redaction, source attribution, ACL evidence,
freshness, cursor expiry, refresh state, provider capability hash, and replay
pointers. Raw provider DSL, raw documents, unbounded snippets, credentials, and
private corpus data are rejected at SDK and observability boundaries.

## Existing Macaca Platform Inventory

- `ServiceDescriptor`, domain-pack registration, and `SystemFacade` provide the
  descriptor/Fascade basis for search discovery.
- `macaca-kernel::service_call` provides the traced Command path required for
  all future `search.*` calls.
- Existing unavailable clients/providers provide Null Object behavior for absent
  optional services.
- Memory, persistence, and event-log snapshot patterns can inform search cursor,
  refresh, stats, and replay diagnostics, but there is no completed search
  provider, DTO set, SDK helper, or developer guide yet.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
