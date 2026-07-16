## ADDED Requirements

### Requirement: Macaca SHALL provide Knowledge Summarization Pack as a serviceized capability

Macaca SHALL provide `pack.knowledge.summarization.v1` as a provider-neutral
industrial pack for extractive summarization, abstractive summarization, hybrid
summarization, multi-document synthesis, conversation summaries, context
compression, summary refinement, citation/evidence attachment, comparison,
quality evaluation, evidence inspection, provider capability inspection, and
unavailable diagnostics. Applications SHALL declare the pack in manifests,
admission SHALL resolve it into effective capabilities, and all operations SHALL
run through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.knowledge.summarization.v1` as required and a summarization service provider is registered, healthy, entitled, mode-compatible, source-compatible, evidence-compatible where requested, quota-compatible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, summary modes, source kind support, language support, permission scopes, policy templates, resource limits, health, diagnostics, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing credentials, raw source documents, raw prompts, raw model outputs, raw provider payloads, raw private spans, raw manifests, package bytes, private keys, signatures, or unbounded summaries

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.knowledge.summarization.v1` as required but provider, summary mode, source kind, citation/evidence support, permission, entitlement, approval, resource budget, language support, or host support is absent
- **THEN** admission SHALL block readiness with structured unavailable, denied, unsupported, validation, approval-required, conflict, quota, timeout, or failure diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, summarize with another provider implicitly, drop evidence requirements, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.knowledge.summarization.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL return Null Object unavailable diagnostics instead of creating callable service calls

### Requirement: Summarization commands SHALL use typed canonical service calls

Every `pack.knowledge.summarization.v1` operation SHALL be represented as a
typed command/result DTO and SHALL traverse the canonical service runtime path
with trace, policy, source access, resource, entitlement, approval, health,
snapshot, redaction, replay, and structured error behavior.

#### Scenario: Summary request is planned
- **WHEN** `summarization.plan` is invoked with source handles, source selectors, summary mode, output schema, evidence policy, chunking strategy, synthesis strategy, redaction profile, quality thresholds, and resource budget
- **THEN** Macaca SHALL validate source access, mode support, provider capability, language support, evidence support, redaction, estimated cost, and resource limits without generating summary output
- **AND** it SHALL return a typed summary plan, chunk plan, estimated cost, estimated latency, risk flags, unavailable diagnostics, and replay pointer

#### Scenario: Summary is generated
- **WHEN** `summarization.summarize` is invoked with a valid summary request and policy-admissible source handles
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and summarization service provider
- **AND** it SHALL return a typed summary output, summary handle, version hash, quality report handle, evidence map where requested, and sanitized replay pointer

#### Scenario: Cited summary requires evidence capability
- **WHEN** `summarization.summarize_with_citations` is invoked with an evidence policy requiring citations or source anchors
- **THEN** Macaca SHALL validate citation/evidence capability and source span access before generation
- **AND** if the capability is absent it SHALL return structured unavailable diagnostics rather than producing uncited text

#### Scenario: Command is denied before provider call
- **WHEN** policy, source access, permission, entitlement, approval, resource, output schema, evidence, language, redaction, or quality-threshold checks reject a `summarization.*` command
- **THEN** Macaca SHALL return a typed denied, approval-required, validation, conflict, quota, timeout, unavailable, or unsupported result before invoking the concrete provider
- **AND** audit evidence SHALL include bounded reason codes without raw credentials, raw source documents, raw prompts, raw model outputs, raw provider payloads, raw private spans, private conversation text, or unbounded summaries

### Requirement: Summarization DTOs SHALL model sources, plans, outputs, claims, evidence, compression maps, comparisons, quality, and provider capability

`pack.knowledge.summarization.v1` SHALL define portable DTOs for summary
sources, summary requests, summary plans, summary outputs, claims, evidence
links, compression maps, comparison reports, quality reports, provider
capabilities, streaming results, partial results, and diagnostics.
Provider-specific fields SHALL remain bounded adapter metadata and SHALL NOT
become OS-layer routing branches.

#### Scenario: Developer inspects summary request schema
- **WHEN** SDK schemas expose `SummaryRequest`
- **THEN** the schema SHALL identify summary mode, target audience class, length policy, tone policy, output schema, language policy, evidence policy, quote policy, freshness policy, chunking strategy, synthesis strategy, resource budget, quality thresholds, and redaction profile
- **AND** provider-specific prompts or model names SHALL NOT be required for portable application logic

#### Scenario: Developer inspects summary output schema
- **WHEN** SDK schemas expose `SummaryOutput`
- **THEN** the schema SHALL include summary handle, generated text handle, structured sections, extracted spans, claims, citations, evidence map, language, version hash, confidence, quality report handle, and redaction profile
- **AND** raw source text and raw model output SHALL NOT be exposed in traces, audits, snapshots, or SDK diagnostics unless policy explicitly permits bounded snippets

#### Scenario: Developer inspects compression map schema
- **WHEN** SDK schemas expose `CompressionMap`
- **THEN** the schema SHALL include original context handles, retained fact handles, compressed summary handle, dropped span hashes, unresolved item handles, decision handles, risk flags, and recovery notes handle
- **AND** it SHALL provide replayable evidence of what was retained, dropped, or marked risky without logging raw private context

#### Scenario: Provider reports capability limits
- **WHEN** SDK discovery inspects the active summarization provider
- **THEN** Macaca SHALL report modes, languages, max source size, max output size, streaming support, citation support, extractive span support, structured-output support, context-compression support, evaluation support, rate limits, lifecycle, health, and capability hash
- **AND** callers SHALL use this metadata instead of provider-name branches

### Requirement: Summarization modes SHALL preserve source, evidence, and quality semantics

`pack.knowledge.summarization.v1` SHALL distinguish extractive, abstractive,
hybrid, multi-document, conversation, meeting, rolling, and context-compression
modes. Each mode SHALL declare source, evidence, output, quality, and resource
semantics so applications can reason about faithfulness and loss.

#### Scenario: Extractive summary preserves source spans
- **WHEN** `summarization.summarize` runs in extractive mode
- **THEN** Macaca SHALL return extracted span handles, source order or salience metadata, coverage diagnostics, and evidence links for selected content
- **AND** it SHALL NOT claim generated paraphrases as extracted source text

#### Scenario: Abstractive summary returns generated claims
- **WHEN** `summarization.summarize` runs in abstractive mode
- **THEN** Macaca SHALL return generated summary handles, claim handles, source support status, quality diagnostics, and unsupported-claim counts according to provider capability
- **AND** it SHALL distinguish generated text from source-extracted spans

#### Scenario: Multi-document synthesis handles partial failures
- **WHEN** `summarization.summarize_many` processes multiple source handles with chunking and synthesis
- **THEN** Macaca SHALL return partial summary handles, synthesis status, skipped source diagnostics, partial failure records, and final summary status
- **AND** it SHALL NOT silently omit unavailable or denied sources

#### Scenario: Conversation summary protects speaker data
- **WHEN** `summarization.summarize_conversation` processes messages, transcript segments, speaker turns, timestamps, or action-item requests
- **THEN** Macaca SHALL treat participants, speaker names, timestamps, and message content as policy-sensitive source data
- **AND** it SHALL return redacted speaker/action metadata according to permission and policy

#### Scenario: Context compression preserves recovery-critical state
- **WHEN** `summarization.compress_context` compresses agent/session context
- **THEN** Macaca SHALL preserve unresolved tasks, decisions, constraints, approvals, blockers, open questions, recovery notes, retained facts, and risk flags according to policy
- **AND** it SHALL return a compression map that supports later audit and recovery diagnostics

### Requirement: Summarization Pack SHALL enforce permissions, source access, resource limits, entitlements, approvals, and redaction

`pack.knowledge.summarization.v1` SHALL define permission scopes for planning,
summary generation, citations, context compression, conversation summaries,
refinement, comparison, evaluation, evidence reading, and provider inspection.
Policy SHALL run before side effects and SHALL account for source access,
summary mode, evidence policy, language, output format, context retention,
provider quota, cost, approval, and redaction.

#### Scenario: Source permission is missing
- **WHEN** an application has summarization permission but lacks source access for a document, retrieval item, citation, graph item, message, transcript, or prior summary
- **THEN** Macaca SHALL return a typed denied result before planning or generation
- **AND** the concrete provider SHALL NOT be invoked

#### Scenario: Evidence permission is missing
- **WHEN** an application can generate summaries but lacks `summarization.evidence.read`
- **THEN** `summarization.inspect_summary_evidence` SHALL return a typed denied result or redacted evidence according to policy
- **AND** source spans, citation handles, graph provenance handles, and claim support details SHALL NOT leak through traces, audits, snapshots, or SDK diagnostics

#### Scenario: Resource limits reject long summary job
- **WHEN** a summary job exceeds max input, max output, chunk count, partial summary count, streaming output, timeout, memory, storage, network, provider quota, evaluation cost, or retained snapshot limits
- **THEN** Macaca SHALL return typed quota, timeout, cancellation, or partial-result diagnostics
- **AND** it SHALL emit bounded resource counters and stable reason codes

#### Scenario: Sensitive external disclosure requires approval
- **WHEN** a summary would disclose sensitive source text outside the application boundary, export to an external destination, compress irreversible session state, or run a long/expensive job
- **THEN** Macaca SHALL return an approval-required result before side effects until a valid approval token is supplied
- **AND** trace/audit evidence SHALL record the approval decision without exposing raw source text

### Requirement: Summary comparison, evaluation, and evidence inspection SHALL be typed and bounded

`pack.knowledge.summarization.v1` SHALL support comparing summaries,
evaluating summary quality, and inspecting summary evidence through typed,
bounded, policy-checked commands. These commands SHALL expose diagnostics without
claiming more certainty than supported by provider capability and source
evidence.

#### Scenario: Summaries are compared
- **WHEN** `summarization.compare_summaries` is invoked with two or more summary handles
- **THEN** Macaca SHALL validate read permission, source/evidence access, compatibility, and resource limits
- **AND** it SHALL return coverage matrix, contradiction set, drift score, missing evidence, duplicate claims, freshness gaps, and recommendations handle

#### Scenario: Summary is evaluated
- **WHEN** `summarization.evaluate_summary` is invoked with summary handle, source handles, and quality thresholds
- **THEN** Macaca SHALL evaluate coverage, faithfulness, concision, relevance, redundancy, sensitivity, unsupported claims, citation coverage, freshness, and provider diagnostics according to capability
- **AND** it SHALL return unsupported/partial diagnostics when a metric is not available

#### Scenario: Evidence is inspected
- **WHEN** `summarization.inspect_summary_evidence` is invoked
- **THEN** Macaca SHALL validate evidence permission and redaction policy
- **AND** it SHALL return summary spans, source span handles, citation handles, retrieval handles, graph provenance handles, support scores, and replay pointers without raw private source text unless policy permits bounded snippets

### Requirement: Summarization Pack SHALL expose industrial metadata and developer documentation

`pack.knowledge.summarization.v1` SHALL expose descriptor metadata for summary
modes, source kinds, languages, output formats, structured-output support,
citation support, context-compression support, evaluation support, command
schemas, permission scopes, policy templates, resource budgets, approval
requirements, lifecycle state, compatibility, health probes, snapshots,
unavailable diagnostics, redaction profiles, SDK examples, provider capability
hashes, and developer documentation.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.knowledge.summarization.v1`
- **THEN** it SHALL return command namespace `summarization.*`, supported modes, source kinds, languages, output formats, supported commands, permissions, policy templates, citation support, context-compression support, evaluation support, examples, lifecycle, availability, health, diagnostics, compatibility, redaction profile, provider capability hash, and documentation links
- **AND** examples SHALL use generic handles and synthetic data rather than application-specific workflows, provider names, credentials, private source documents, raw prompts, or domain-specific summary templates

#### Scenario: Developer documentation is published
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/knowledge/summarization.md` SHALL document manifest declaration, required versus optional behavior, permissions, source handles, source selectors, extractive/abstractive/hybrid modes, multi-document synthesis, conversation summaries, context compression, chunking strategies, evidence links, citation handles, claims, quality reports, comparison reports, compression maps, unavailable diagnostics, provider replacement, trace/audit interpretation, operational limits, and conformance tests
- **AND** SDK discovery metadata and the industrial catalog index SHALL link to that guide

### Requirement: Summarization Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.knowledge.summarization.v1` SHALL emit sanitized trace/audit events and
bounded snapshots for declaration, admission, planning, request validation,
summary generation, cited summary generation, multi-document synthesis,
conversation summaries, context compression, refinement, comparison, evaluation,
evidence inspection, provider inspection, policy/resource decisions, provider
calls, unavailable states, and replay.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a summarization pack snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, supported modes, source kind support, language support, citation/evidence support, context-compression support, evaluation support, command availability, provider health, policy template hash, resource counters, bounded quality aggregates, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, raw source documents, raw prompts, raw model outputs, raw provider payloads, raw private spans, private conversation text, unbounded summaries, manifests, package bytes, private keys, and signatures

#### Scenario: Summary generation is audited
- **WHEN** `summarization.summarize`, `summarization.summarize_with_citations`, `summarization.summarize_many`, or `summarization.summarize_conversation` runs
- **THEN** Macaca SHALL emit sanitized audit events with source inventory hash, summary mode, capability hash, policy decision, resource counters, result status, quality report handle, evidence-map status, latency, and replay pointer
- **AND** raw source documents, raw prompts, raw model outputs, raw provider payloads, and unbounded summaries SHALL NOT enter audit records

#### Scenario: Context compression is audited
- **WHEN** `summarization.compress_context` runs
- **THEN** Macaca SHALL emit sanitized audit events with context inventory hash, retained fact count, dropped span hash count, unresolved item count, decision handle count, risk flags, result code, and replay pointer
- **AND** raw private context and private conversation text SHALL NOT enter audit records

### Requirement: Summarization Pack implementation SHALL preserve Macaca boundaries

The `pack.knowledge.summarization.v1` implementation SHALL remain owned by
summarization service providers behind the service runtime. The microkernel,
SDK, shells, and generic application framework SHALL remain provider-neutral and
free of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete OpenAI, Anthropic, Azure, Google, AWS, local model, prompt-template, model-client, or provider adapter imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.knowledge.summarization.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hash, and result codes rather than provider-specific business branches

#### Scenario: SDK helper builds service call only
- **WHEN** an SDK helper such as `sdk.packs.knowledge.summarization.summarize(command)` is used
- **THEN** the helper SHALL build a canonical traced service call with command DTO, permission metadata, source handles, resource limits, redaction profile, and replay context
- **AND** it SHALL NOT construct providers, build raw prompts as OS semantics, instantiate model clients, route by provider name, or bypass policy
