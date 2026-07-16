# Change: Add Industrial Knowledge Search Pack

## Why

Applications need search as a reusable knowledge capability across declared
corpora. Industrial search is not just keyword matching: it includes corpus
registration, schema/index metadata, analyzers, filters, facets, sorting,
ranking profiles, pagination, suggestions, highlighting, access-control
trimming, freshness, incremental indexing, explainability, query auditing, and
provider replacement.

If every application builds its own search path, Macaca will accumulate
application-specific indexes, duplicated provider integrations, inconsistent
permission checks, and non-replayable results. This pack defines one canonical
service path for provider-neutral search.

## Supplier And Platform API Research

This proposal maps established search provider concepts into Macaca
abstractions:

- Elasticsearch and OpenSearch expose indices, mappings, analyzers, query DSL,
  filters, aggregations, sorting, pagination/search-after, highlighting,
  explain/profile, refresh, aliases, snapshots, and security filtering. Macaca
  maps these to corpus descriptors, field schemas, analyzer profiles, query AST,
  facet requests, sort specs, cursor pagination, snippet metadata, ranking
  explanation, refresh command, snapshot metadata, and ACL trimming.
- Algolia exposes searchable attributes, custom ranking, filters, facets,
  typo-tolerance, synonyms, replicas, query suggestions, analytics, and secured
  API keys. Macaca maps these to ranking profiles, filter/facet DTOs, synonym
  sets, suggestion commands, analytics counters, and policy-scoped query tokens
  represented as secret references.
- Azure AI Search exposes indexes, indexers, skillsets, semantic search,
  filters, facets, scoring profiles, suggest/autocomplete, vector/hybrid
  search, analyzers, index statistics, and document-level security patterns.
  Macaca maps these to index capability descriptors, scoring profiles,
  semantic/hybrid support metadata, suggest/autocomplete commands, index
  statistics, and permission-aware result filtering.
- Microsoft Graph Search and Google Programmable Search expose federated search
  across declared data sources, result templates, verticals, query constraints,
  and source attribution. Macaca maps these to corpus handles, provider
  verticals, result source attribution, and provider-query capability metadata.

The Macaca contract is not a pass-through query DSL. It provides a stable query
AST and capability metadata. Provider-specific query options are bounded adapter
metadata and never become OS-layer branches.

## What Changes

- Add provider-neutral `pack.knowledge.search.v1` under the `knowledge` family.
- Define corpus, index, schema, field, analyzer, synonym, ranking profile,
  query AST, filter, facet, sort, pagination cursor, hit, snippet, source
  attribution, ACL, index refresh, and explanation DTOs.
- Define commands for registering corpora, inspecting indexes, search, suggest,
  autocomplete, facets, explain ranking, refresh index, index statistics, and
  query diagnostics.
- Define permission scopes for corpus registration, search, suggest, metadata
  inspection, refresh, explain, statistics, and admin-only schema access.
- Require ACL trimming, source attribution, sanitized snippets, bounded result
  pages, query/resource budgets, replayable result provenance, unavailable
  diagnostics, and a detailed developer guide.

## Impact

- Affected specs: `pack-knowledge-search`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected future code: provider-neutral proto DTOs, search descriptors,
  admission validators, SDK discovery metadata, focused SDK clients, search
  service providers, unavailable/mock providers, trace/audit schemas, query
  parser/AST validation, replay tests, and dependency-boundary gates.
- Non-goals: no application-specific corpus logic, no provider-name routing in
  OS layers, no raw index/provider payload exposure, no concrete provider
  construction in kernel/SDK/shells, and no fake success when search providers
  or indexes are unavailable.

## References

- Elasticsearch Search API:
  https://www.elastic.co/guide/en/elasticsearch/reference/current/search-search.html
- Elasticsearch Query DSL:
  https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl.html
- OpenSearch Search API:
  https://opensearch.org/docs/latest/api-reference/search/
- Algolia Search API:
  https://www.algolia.com/doc/api-reference/search-api-parameters/
- Algolia filtering/faceting:
  https://www.algolia.com/doc/guides/managing-results/refine-results/filtering/
- Azure AI Search:
  https://learn.microsoft.com/en-us/azure/search/search-what-is-azure-search
- Azure AI Search query:
  https://learn.microsoft.com/en-us/azure/search/search-query-overview
- Microsoft Graph Search:
  https://learn.microsoft.com/en-us/graph/search-concept-overview
- Google Programmable Search:
  https://developers.google.com/custom-search/v1/overview
