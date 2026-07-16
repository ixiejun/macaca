## ADDED Requirements

### Requirement: Macaca SHALL provide Knowledge Search Pack as a serviceized capability

Macaca SHALL provide `pack.knowledge.search.v1` as a provider-neutral industrial
pack for corpus registration, indexed/federated search, suggestions, facets,
ranking explanations, index inspection, refresh, statistics, ACL trimming,
source attribution, and unavailable diagnostics. Applications SHALL declare the
pack in manifests, admission SHALL resolve it into effective capabilities, and
all operations SHALL run through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.knowledge.search.v1` as required and a search service provider is registered, healthy, entitled, corpus-compatible, ACL-compatible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, corpus capability metadata, permission scopes, policy templates, query feature support, health, diagnostics, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing raw credentials, raw query tokens, raw provider payloads, raw documents, or private corpus content

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.knowledge.search.v1` as required but provider, corpus support, permission, entitlement, ACL model, resource budget, or policy support is absent
- **THEN** admission SHALL block readiness with structured unavailable, denied, unsupported, validation, or quota diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.knowledge.search.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL return Null Object unavailable diagnostics instead of creating callable service calls

### Requirement: Search commands SHALL use typed canonical service calls

Every `pack.knowledge.search.v1` operation SHALL be represented as a typed
command/result DTO and SHALL traverse the canonical service runtime path with
trace, policy, resource, entitlement, approval, health, snapshot, query
validation, ACL trimming, redaction, replay, and structured error behavior.

#### Scenario: Query is executed
- **WHEN** `search.search` is invoked with a corpus handle, provider-neutral query AST, filters, sort, page size, snippet policy, and attribution request
- **THEN** Macaca SHALL validate corpus access, ACL trimming, query complexity, provider capability, resource budget, and redaction policy before invoking the provider
- **AND** it SHALL return typed hits, snippets, source attribution, cursor metadata, result counts where supported, and sanitized replay evidence

#### Scenario: Suggestions are requested
- **WHEN** `search.suggest` or `search.autocomplete` is invoked with a bounded prefix or query context
- **THEN** Macaca SHALL enforce suggestion permission, sensitive-term policy, result limits, provider capability, and rate limits
- **AND** unsupported suggestion features SHALL return typed unsupported diagnostics rather than provider-specific errors

#### Scenario: Ranking is explained
- **WHEN** `search.explain_ranking` is invoked for a query and hit handle
- **THEN** Macaca SHALL require explain permission and return redacted ranking profile, matched fields, bounded feature metadata, and confidence
- **AND** raw provider explanations, private field values, raw documents, and unbounded scoring traces SHALL NOT be exposed

#### Scenario: Command is denied before provider call
- **WHEN** policy, permission, entitlement, ACL, approval, resource, query validation, or redaction checks reject a search command
- **THEN** Macaca SHALL return a typed denied, validation, or quota result before invoking the concrete provider
- **AND** audit evidence SHALL include a bounded reason code without raw documents, snippets beyond policy, query tokens, credentials, or provider payloads

### Requirement: Search DTOs SHALL model corpora, schemas, queries, facets, hits, cursors, ACLs, and explanations

`pack.knowledge.search.v1` SHALL define portable DTOs for searchable corpora,
index schemas, fields, analyzer profiles, synonym sets, ranking profiles, query
AST, filters, facets, sorting, hits, cursors, ranking explanations, ACL evidence,
provider capability, and diagnostics. Provider-specific fields SHALL remain
bounded adapter metadata and SHALL NOT become OS-layer routing branches.

#### Scenario: Developer inspects search schema
- **WHEN** SDK discovery or `search.inspect_index` exposes a schema
- **THEN** the schema SHALL include field definitions, searchable/filterable/facetable/sortable flags, analyzer profiles, synonym sets, semantic/hybrid support metadata, redaction policy, freshness, health, and compatibility
- **AND** raw documents, provider topology beyond policy, credentials, and private corpus content SHALL NOT be exposed

#### Scenario: Developer requests facets
- **WHEN** `search.facets` is invoked
- **THEN** Macaca SHALL validate facet permission, declared facetable fields, bucket limits, cardinality budget, privacy thresholds, and provider capability
- **AND** buckets that would leak private or low-cardinality sensitive data SHALL be suppressed or redacted according to policy

#### Scenario: Provider reports capability limits
- **WHEN** SDK discovery inspects the active search provider
- **THEN** Macaca SHALL report query AST features, filters, facets, sort, suggest, autocomplete, semantic/hybrid support, explain depth, refresh support, page limits, rate limits, lifecycle, and health
- **AND** callers SHALL use this metadata rather than provider-name branches

### Requirement: Search Pack SHALL enforce permissions, ACL trimming, redaction, and resource budgets

`pack.knowledge.search.v1` SHALL define permission scopes for search queries,
suggestions, facets, ranking explanation, index inspection, index refresh,
corpus management, and statistics. Policy SHALL run before side effects and
SHALL account for corpus ownership, ACL trimming, source attribution, query
complexity, page size, cursor expiry, snippet redaction, provider capability,
refresh quota, and resource budgets.

#### Scenario: ACL trimming removes unauthorized hits
- **WHEN** a search provider returns results that include documents outside the caller's corpus or document ACL scope
- **THEN** Macaca SHALL remove or redact unauthorized hits before returning them
- **AND** trace/audit evidence SHALL record bounded ACL-trimming counters without revealing unauthorized document details

#### Scenario: Deep pagination is rejected
- **WHEN** a query requests an offset, page size, or cursor pattern that exceeds policy or provider limits
- **THEN** Macaca SHALL return a typed quota or validation result with bounded diagnostics
- **AND** it SHALL recommend cursor/search-after pagination when available

#### Scenario: Index refresh is requested
- **WHEN** `search.refresh_index` is invoked
- **THEN** Macaca SHALL require refresh permission, provider capability, quota budget, and async handle support
- **AND** expensive refresh or reindex operations SHALL be traceable and cancellable when provider capability allows

### Requirement: Search Pack SHALL expose industrial metadata and developer documentation

`pack.knowledge.search.v1` SHALL expose descriptor metadata for corpus
capabilities, command schemas, permission scopes, policy templates, query feature
support, filter/facet/sort limits, ranking profiles, semantic/hybrid support,
resource budgets, SDK examples, lifecycle state, compatibility, health probes,
snapshots, unavailable diagnostics, redaction profiles, and developer
documentation.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.knowledge.search.v1`
- **THEN** it SHALL return command namespace `search.*`, corpus capabilities, supported commands, permissions, policy templates, query feature support, facet/sort limits, ranking profiles, examples, lifecycle, availability, health, diagnostics, compatibility, redaction profile, and documentation links
- **AND** examples SHALL use generic handles and synthetic data rather than application-specific workflows, provider names, credentials, or business routing

#### Scenario: Developer documentation is published
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/knowledge/search.md` SHALL document manifest declaration, permissions, corpus registration, schema design, query AST, filters, facets, sorting, pagination, suggestions, autocomplete, ranking profiles, ACL trimming, snippets, source attribution, refresh, explainability, provider replacement, unavailable diagnostics, trace/audit interpretation, and operational limits
- **AND** SDK discovery metadata and the industrial catalog index SHALL link to that guide

### Requirement: Search Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.knowledge.search.v1` SHALL emit sanitized trace/audit events and bounded
snapshots for declaration, admission, corpus registration, index inspection,
queries, suggestions, facets, ranking explanations, refresh, statistics,
policy/resource decisions, provider calls, unavailable states, and replay.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a search pack snapshot
- **THEN** the snapshot SHALL include descriptor version, corpus capability hashes, index health, schema hashes, ranking profile hashes, freshness summaries, provider health, command availability, policy template hash, resource counters, and sanitized replay pointers
- **AND** it SHALL exclude raw documents, raw snippets beyond policy, raw provider payloads, credentials, raw query tokens, private corpus content, and unbounded output

#### Scenario: Query is audited
- **WHEN** a search, suggest, autocomplete, facet, explain, refresh, or diagnostics command runs
- **THEN** Macaca SHALL emit a sanitized audit event with stable corpus handles, command name, query hash, policy decision, ACL-trimming counters, result count bounds, provider capability hash, result code, and replay pointer
- **AND** the event SHALL exclude raw sensitive query text when policy requires hashing or redaction

### Requirement: Search implementation SHALL preserve Macaca boundaries

The `pack.knowledge.search.v1` implementation SHALL remain owned by knowledge
search service providers behind the service runtime. The microkernel, SDK,
shells, and generic application framework SHALL remain provider-neutral and free
of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete search provider or index adapter imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.knowledge.search.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hash, and result codes rather than provider-specific business branches
