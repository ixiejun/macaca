## 1. Supplier API Research And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries,
  serviceization allowlist, design-pattern guidance, and the industrial catalog
  umbrella proposal before implementation.
- [x] 1.2 Record API notes for Elasticsearch/OpenSearch search APIs, Algolia
  search/filter/facet/ranking APIs, Azure AI Search query/scoring/semantic APIs,
  Microsoft Graph Search, and Google Programmable Search.
- [x] 1.3 Map supplier concepts to provider-neutral corpus, index schema, field,
  analyzer, synonym, ranking profile, query AST, filter, facet, sort, cursor,
  hit, snippet, source attribution, ACL, refresh, stats, and explanation DTOs.
- [x] 1.4 Inventory existing service descriptors, SDK clients, admission paths,
  trace/audit schemas, optional providers, mock providers, unavailable providers,
  storage/indexing primitives, and policy/resource gates that can back search.
- [x] 1.5 Record GitNexus CRITICAL/HIGH findings as memo only before
  implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define provider-neutral DTOs for `SearchCorpus`,
  `SearchIndexSchema`, `SearchField`, `SearchAnalyzerProfile`,
  `SearchSynonymSet`, `SearchRankingProfile`, `SearchQuery`, `SearchFilter`,
  `SearchFacetRequest`, `SearchSort`, `SearchHit`, `SearchCursor`,
  `SearchRankingExplanation`, and `SearchProviderCapability`.
- [x] 2.2 Define typed command DTOs for `search.register_corpus`,
  `search.inspect_index`, `search.search`, `search.suggest`,
  `search.autocomplete`, `search.facets`, `search.explain_ranking`,
  `search.refresh_index`, `search.index_stats`, and
  `search.query_diagnostics`.
- [x] 2.3 Define typed success, page, async-handle, denied, unavailable,
  unsupported, conflict, quota, timeout, validation, and provider-failure result
  DTOs.
- [x] 2.4 Define descriptor metadata for pack id, corpus types, command schemas,
  permissions, policy templates, query feature support, filter/facet/sort limits,
  ranking profiles, semantic/hybrid support, ACL model, redaction profile, SDK
  metadata, compatibility, diagnostics, and documentation links.
- [x] 2.5 Add descriptor hash, query AST validation, schema compatibility,
  ACL-trimming, redaction-profile, and provider capability tests.

## 3. Admission, Permission, Policy, Resource, And Approval

- [x] 3.1 Implement declaration validation for scopes:
  `knowledge.search.query`, `knowledge.search.suggest`,
  `knowledge.search.facets`, `knowledge.search.explain`,
  `knowledge.search.index.read`, `knowledge.search.index.refresh`,
  `knowledge.search.corpus.manage`, and `knowledge.search.stats`.
- [ ] 3.2 Enforce corpus ownership, ACL trimming, source attribution, query
  complexity, page size, cursor expiry, timeout, facet cardinality, snippet
  redaction, provider capability, rate limit, refresh quota, and resource budget
  checks before provider calls.
- [ ] 3.3 Reject raw credentials, raw provider payloads, raw documents,
  unbounded snippets, raw query tokens, private corpus content, and unbounded
  output at admission and observability boundaries.
- [x] 3.4 Model required declarations as readiness blockers and optional
  declarations as explicit degraded effective capabilities.
- [ ] 3.5 Add tests proving denied, validation, quota, unsupported, and
  unavailable paths do not call concrete search providers.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Implement or bind search providers only through the service runtime and
  approved runtime-host composition roots.
- [ ] 4.2 Add unavailable and mock providers with deterministic corpus, schema,
  query, facet, suggest, explain, refresh, stats, and capability behavior.
- [x] 4.3 Add lifecycle, health, snapshot, shutdown, timeout, cancellation,
  bounded pagination, cursor resume, query diagnostics, refresh async handles,
  and index health support.
- [ ] 4.4 Add provider capability reporting for query AST features, filters,
  facets, sort, suggest, autocomplete, semantic/hybrid support, explain depth,
  refresh support, page limits, rate limits, and health.
- [x] 4.5 Add canonical execution-path tests proving every search command
  traverses SDK/facade, service runtime decorators, and provider dispatch exactly
  once.

## 5. SDK, WASM ABI, Application Framework, And Examples

- [x] 5.1 Extend SDK discovery for `pack.knowledge.search.v1` with command
  schemas, corpus capability reports, examples, availability, diagnostics, docs
  metadata, policy templates, query feature support, facet/sort limits, ranking
  profiles, and compatibility.
- [x] 5.2 Add focused SDK helper builders that only produce canonical traced
  service calls and return Null Object unavailable diagnostics when the pack is
  absent.
- [ ] 5.3 Extend WASM/application ABI metadata so applications can declare corpus
  search access, inspect indexes, run queries, request facets, and explain
  ranking only through declared permissions.
- [x] 5.4 Add generic examples for register corpus, inspect index, keyword
  search, structured filters, facets, sort, cursor pagination, suggestions,
  autocomplete, explain ranking, refresh index, diagnostics, and unavailable
  provider handling.

## 6. Trace, Audit, Replay, Security, And Gates

- [ ] 6.1 Emit sanitized declaration, admission, corpus registration, index
  inspection, query, suggestion, facet, ranking explanation, refresh, stats,
  policy, resource, entitlement, approval, service-call, provider-call, health,
  snapshot, and unavailable events.
- [x] 6.2 Add replay tests proving corpus registration, index inspection, search,
  cursor pagination, facets, suggestions, explain ranking, refresh, and query
  diagnostics are trace-addressable through the canonical service path.
- [ ] 6.3 Add sanitization tests proving traces, audits, snapshots, SDK
  diagnostics, and examples do not leak raw credentials, raw provider payloads,
  raw documents, private corpus content, raw query tokens, or unbounded snippets.
- [x] 6.4 Add dependency gates proving kernel, SDK, shells, and generic
  application framework do not import concrete search providers or index
  adapters.
- [x] 6.5 Run `openspec validate add-pack-knowledge-search --strict`, targeted
  cargo tests, boundary gates, file-size gates, canonical execution-path tests,
  and audit replay checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/knowledge/search.md` with pack purpose,
  platform comparison, manifest declaration, permission scopes, corpus
  registration, index schema, query AST, filters, facets, sorting, pagination,
  suggestions, autocomplete, ranking profiles, ACL trimming, snippets, source
  attribution, refresh, explainability, provider replacement, unavailable
  diagnostics, trace/audit interpretation, and operational limits.
- [x] 7.2 Include generic app-facing examples for register corpus, inspect index,
  search, filter, facet, sort, paginate, suggest, autocomplete, explain ranking,
  refresh index, inspect stats, and handle unavailable provider results.
- [x] 7.3 Include provider-author guidance for descriptor metadata, schema
  mapping, analyzer profiles, ACL enforcement, redaction, ranking explanations,
  snapshots, quota reporting, and conformance tests.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial
  pack catalog index before marking `add-pack-knowledge-search` complete.
