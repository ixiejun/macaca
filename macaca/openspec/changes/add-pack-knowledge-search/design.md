# Knowledge Search Pack Design

## Context

`pack.knowledge.search.v1` exposes indexed and federated search as a Macaca OS
serviceized capability. It lets applications search declared corpora through a
provider-neutral contract while concrete search engines remain replaceable
providers.

Search is high-risk for data leakage: a query can reveal corpus existence,
snippets can expose private content, and ranking explanations can leak field
weights or access-control metadata. The pack therefore treats corpus access,
query execution, suggestions, index inspection, refresh, and explainability as
typed service commands protected by policy, ACL trimming, resource budgets,
redaction, and replayable audit evidence.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Elasticsearch/OpenSearch | Indices, mappings, analyzers, query DSL, filters, aggregations, sort, search-after, highlights, explain/profile, refresh, snapshots | Corpus/index descriptor, field schema, analyzer profile, query AST, facets, sort spec, cursor, snippet, ranking explanation, refresh, snapshot |
| Algolia | Searchable attributes, custom ranking, filters, facets, typo tolerance, synonyms, replicas, suggestions, analytics, secured keys | Ranking profile, typo/synonym policy, facet/filter DTOs, suggestion command, analytics counters, policy-scoped secret references |
| Azure AI Search | Indexes, indexers, analyzers, scoring profiles, semantic search, vector/hybrid support, suggest/autocomplete, statistics | Index capability descriptor, scoring profile, semantic/hybrid capability metadata, suggest/autocomplete, statistics |
| Microsoft Graph Search | Federated search, verticals, result types, query constraints, ACL-aware results | Corpus vertical, result type, source attribution, ACL trimming |
| Google Programmable Search | Programmable engines, query parameters, refinements, source attribution | Provider query capability, corpus handle, refinements, attribution |

## Goals

- Provide stable pack id `pack.knowledge.search.v1` and command namespace
  `search.*`.
- Support corpus registration, index inspection, search, suggest, autocomplete,
  facet retrieval, ranking explanation, index refresh, statistics, and query
  diagnostics.
- Model query AST, filters, facets, sorting, pagination cursor, snippets,
  source attribution, ACL trimming, freshness, and ranking profiles explicitly.
- Keep provider-specific query syntax in bounded adapter options, never in
  OS-layer routing branches.
- Require developer documentation under `docs/developer-packs/knowledge/search.md`.

## Non-Goals

- Do not implement a concrete Elasticsearch, OpenSearch, Algolia, Azure,
  Graph, Google, or local search provider in this proposal.
- Do not implement document parsing, retrieval augmentation, citations, graph
  knowledge, or summarization; those are separate knowledge packs.
- Do not expose raw index documents, raw provider payloads, raw query tokens,
  credentials, private corpus content, or unbounded snippets in logs, traces,
  snapshots, SDK diagnostics, or examples.
- Do not let shells or applications bypass search service policy with direct
  provider query strings.

## Ownership And Boundaries

- Pack id: `pack.knowledge.search.v1`.
- Family: `knowledge`.
- Backing service owner: knowledge search service provider.
- SDK surface: `sdk.packs.knowledge.search`.
- Command namespace: `search.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, service decorators, indexer
  bridge composition, and sanitized diagnostics through approved composition
  roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `search.register_corpus` | Register searchable corpus metadata and provider binding | Requires entitlement, schema policy, ACL model, and secret references |
| `search.inspect_index` | Inspect corpus/index schema, capabilities, freshness, and health | Returns bounded metadata, never raw documents |
| `search.search` | Execute provider-neutral query AST | Requires ACL trimming, page limit, timeout budget, and redacted snippets |
| `search.suggest` | Return query suggestions or term suggestions | Requires suggestion capability and sensitive-term policy |
| `search.autocomplete` | Return prefix completions | Requires bounded prefix policy and result limits |
| `search.facets` | Return facet buckets or aggregations | Requires facet permission and cardinality budget |
| `search.explain_ranking` | Explain ranking for a query/hit | Requires explain permission and redacted scoring metadata |
| `search.refresh_index` | Request refresh/reindex for declared corpus | Requires refresh permission, quota, and async handle |
| `search.index_stats` | Return index size, freshness, shard/provider health, and quota data | Must avoid raw provider topology leakage beyond policy |
| `search.query_diagnostics` | Validate query AST and report unsupported filters/capabilities | Must not execute provider search side effects |

## DTO Model

Core DTOs:

- `SearchCorpus`: corpus handle, source type, owner scope, schema handle,
  provider class, ACL model, freshness policy, retention policy, indexing state,
  and capability hash.
- `SearchIndexSchema`: field definitions, searchable/filterable/facetable/
  sortable flags, analyzer profiles, synonym sets, vector/semantic support
  metadata, and redaction policy.
- `SearchQuery`: query text, structured query AST, filters, facets, sort,
  ranking profile, locale, time bounds, result size, cursor, snippet policy,
  source attribution request, and explain flag.
- `SearchFilter`: boolean expression over declared fields, ranges, terms,
  existence, geo/time filters when supported, and unsupported-filter diagnostics.
- `SearchFacetRequest`: field, bucket limit, sort policy, min count, privacy
  threshold, and cardinality budget.
- `SearchHit`: result handle, corpus handle, source handle, title, redacted
  snippet, score, rank, highlights, source attribution, freshness, ACL evidence
  hash, and content handles.
- `SearchCursor`: provider-neutral page cursor, search-after token reference,
  result window policy, expiry, and replay pointer.
- `SearchRankingExplanation`: hit handle, ranking profile, feature names,
  bounded weights, matched fields, redacted explanation text, and confidence.
- `SearchProviderCapability`: query features, filters, facets, sort, suggest,
  autocomplete, semantic/hybrid support, explain depth, refresh support, page
  limits, rate limits, and health.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `knowledge.search.query`
- `knowledge.search.suggest`
- `knowledge.search.facets`
- `knowledge.search.explain`
- `knowledge.search.index.read`
- `knowledge.search.index.refresh`
- `knowledge.search.corpus.manage`
- `knowledge.search.stats`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id, and
  trace id when available.
- Search commands require corpus declaration, ACL trimming, page size, timeout,
  query complexity, snippet redaction, and source attribution policy.
- Explain and index inspection require stronger permissions because they can
  leak schema, ranking, or corpus metadata.
- Refresh/reindex requires resource budget, quota, async handle, and approval
  when it can perform expensive or external side effects.
- Provider-specific query options are allowed only through declared bounded
  adapter metadata and must return unsupported diagnostics when unavailable.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
corpus capabilities, permission scopes, policy templates, query feature support,
facet/sort limits, ranking profiles, semantic/hybrid support, examples,
unavailable diagnostics, health, compatibility, redaction profiles, and
documentation links.

The developer guide at `docs/developer-packs/knowledge/search.md` must cover
manifest declarations, corpus registration, schema design, query AST, filters,
facets, sorting, pagination, suggestions, ranking profiles, ACL trimming,
snippets, source attribution, refresh, explainability, unavailable diagnostics,
provider replacement, trace/audit interpretation, and conformance tests.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `search_pack_declared`
- `search_pack_admission_validated`
- `search_corpus_registered`
- `search_index_inspected`
- `search_query_requested`
- `search_query_completed`
- `search_suggest_requested`
- `search_facets_requested`
- `search_ranking_explained`
- `search_index_refresh_requested`
- `search_pack_policy_decision`
- `search_pack_service_call_requested`
- `search_pack_service_call_succeeded`
- `search_pack_service_call_failed`
- `search_pack_unavailable`
- `search_pack_snapshot_recorded`

Snapshots include descriptor version, corpus capability hashes, index health,
schema hashes, ranking profile hashes, freshness summaries, provider health,
command availability, policy template hash, resource counters, and sanitized
replay pointers. Snapshots must exclude raw documents, raw snippets beyond
policy, raw provider payloads, credentials, raw query tokens, private corpus
content, and unbounded output.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, ranking strategies, analyzer support,
  semantic/hybrid support, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  ACL-trimming, and redaction wrap service calls.
- **Specification**: admission validates corpus declarations, schema, query AST,
  permissions, provider capability, and compatibility.
- **Observer**: corpus/index health, query events, refresh events, trace, and
  audit events are subscribable.
- **Memento**: cursors, index snapshots, schema hashes, query replay pointers,
  and refresh handles preserve recovery state.
- **Abstract Factory**: provider adapters are created only by approved runtime
  host composition roots.

## Risks And Mitigations

- Risk: search leaks private corpus content through snippets or facets.
  Mitigation: ACL trimming, privacy thresholds, redaction profiles, and
  sanitized observability are mandatory.
- Risk: query DSL pass-through creates provider lock-in. Mitigation: use a
  provider-neutral query AST and expose provider-specific options only as
  bounded adapter metadata.
- Risk: deep pagination causes expensive provider load. Mitigation: require
  cursor/search-after style pagination, page limits, and query budgets.
- Risk: ranking explanations expose sensitive model/schema details. Mitigation:
  require separate permission and redacted explanation DTOs.
