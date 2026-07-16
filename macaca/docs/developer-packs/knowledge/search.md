# Knowledge Search Pack

`pack.knowledge.search.v1` describes provider-neutral corpus search. The pack
is descriptor-only until a serviceized search provider is installed through the
runtime composition root.

## Manifest Declaration

Declare the pack as required only when search is mandatory for readiness.
Optional declarations degrade with explicit unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.knowledge.search.v1"]
```

## Permissions

Use the narrowest scope: `knowledge.search.query`,
`knowledge.search.suggest`, `knowledge.search.facets`,
`knowledge.search.explain`, `knowledge.search.index.read`,
`knowledge.search.index.refresh`, `knowledge.search.corpus.manage`, and
`knowledge.search.stats`.

## Capability Model

Macaca models search as corpus metadata, index schema, fields, analyzer
profiles, synonym sets, ranking profiles, query envelopes, filters, facets,
sorts, cursors, hits, snippets by reference, source attribution, ACL policy, and
ranking explanations. Raw documents, raw provider query DSL, raw provider
payloads, private corpus content, and unbounded snippets stay outside traces,
snapshots, and SDK diagnostics.

## Platform Comparison

Elasticsearch/OpenSearch concepts map to index schema, field, analyzer, query,
filter, facet, sort, cursor, and explanation DTOs. Algolia ranking, filters,
facets, and replicas map to ranking profile and provider capability metadata.
Azure AI Search query, scoring profile, semantic, and hybrid features map to
query feature and provider capability fields. Microsoft Graph Search and Google
Programmable Search map to federated search capability metadata. Native DSLs
and provider ranking payloads remain provider implementation details.

## Commands

`search.register_corpus`, `search.inspect_index`, `search.search`,
`search.suggest`, `search.autocomplete`, `search.facets`,
`search.explain_ranking`, `search.refresh_index`, `search.index_stats`, and
`search.query_diagnostics` are descriptor-owned schema names. SDK helpers build
canonical traced service calls; providers execute behind the service runtime.

## App-Facing Examples

- Register a corpus with `search.register_corpus` using a corpus handle,
  namespace, ACL policy, and schema reference.
- Inspect an index with `search.inspect_index` before querying.
- Run keyword or structured search with `search.search`; pass query text by
  reference and keep raw tokens out of logs.
- Add filters, facets, and sorting through descriptor-supported fields only.
- Use cursor metadata for pagination and treat cursor expiry as a validation
  failure.
- Request suggestions or autocomplete only when provider capability reports
  support.
- Use `search.explain_ranking` for bounded score explanations and
  `search.query_diagnostics` for trace-safe troubleshooting.
- Handle unavailable results by reading the structured diagnostic reason rather
  than falling back to a provider-specific API.

## Trace And Audit

Traces should record declaration, admission decision, command name, corpus id,
page size, provider class, capability hash, and result status. They must not
record raw query text, raw documents, provider-native query DSL, credentials, or
unbounded snippets.

## Provider Authors

Provider descriptors must report query AST support, filter/facet/sort limits,
semantic or hybrid support, explainability depth, ACL trimming, page limits,
rate limits, redaction behavior, health, and snapshot metadata. Providers must
return structured denied, unsupported, quota, timeout, unavailable, validation,
and provider-failure results without exposing native payloads.

Conformance tests should cover descriptor completeness, schema compatibility,
query-envelope validation, ACL trimming, redaction, pagination, unavailable
behavior, and provider capability reporting.
