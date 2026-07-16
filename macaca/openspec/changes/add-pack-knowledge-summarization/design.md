# Knowledge Summarization Pack Design

## Context

`pack.knowledge.summarization.v1` exposes summarization as a Macaca OS
serviceized capability. It lets applications produce summaries from documents,
retrieval results, conversations, meetings, source spans, graph evidence, or
prior summaries without embedding model prompts, provider SDKs, or
application-specific summary templates into generic OS layers.

Summarization compresses information and can create operational risk: omitted
requirements, hallucinated claims, broken evidence links, sensitive-source leaks,
or lossy context compression. The pack therefore treats summarization as a
typed, policy-checked, evidence-aware service with source handles, redaction,
quality diagnostics, trace/audit evidence, and replayable compression maps.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| OpenAI Responses API and prompting guides | General LLM summarization, instruction-following, structured outputs, context-aware generation | Abstractive/hybrid summary command, provider-neutral generation options, schema-constrained output, quality diagnostics |
| Anthropic context engineering guidance | Context summarization, compaction, preserving unresolved work and key decisions for agents | Context compression command, rolling summary, compression map, retained facts, dropped spans, recovery notes |
| Azure AI Language Summarization | Extractive and abstractive summarization as distinct actions | Summary mode (`extractive`, `abstractive`, `hybrid`), extracted sentence spans, generated summary sentences, mode-specific capability |
| Azure Text Analytics SDK | Abstractive summaries generated as new text rather than extracted concatenation | Generated summary sentence DTO, source coverage and evidence confidence metadata |
| Google Vertex AI samples/codelabs | Generative summarization and long-document strategies such as stuffing, MapReduce, overlapping chunks, and rolling summaries | Summary job plan, chunking strategy, synthesis strategy, partial summaries, final synthesis, long-input diagnostics |
| Amazon Comprehend | NLP insights such as entities, key phrases, events, sentiment, and topics | Extractive evidence signals, salience hints, topic/entity/key-phrase annotations, hybrid summary enrichment |

The pack exposes a provider-neutral contract. Provider adapters may use LLMs,
extractive NLP, rule-based reducers, hybrid pipelines, or remote services, but
OS semantics are descriptor-driven and policy-checked.

## Goals

- Provide stable pack id `pack.knowledge.summarization.v1` and command namespace
  `summarization.*`.
- Support extractive, abstractive, hybrid, multi-document, conversation,
  meeting, rolling, and context-compression summaries.
- Support source constraints, redaction profiles, evidence links, citation
  handles, claim lists, quality scores, coverage, freshness, compression maps,
  refinement, comparison, and provider capability inspection.
- Keep provider/model/prompt-specific behavior behind replaceable service
  providers.
- Require developer documentation at
  `docs/developer-packs/knowledge/summarization.md`.

## Non-Goals

- Do not implement concrete model or cloud provider adapters in this proposal.
- Do not define application-specific summary templates, report formats, legal
  briefs, meeting formats, customer-support workflows, or workflow-specific
  quality rules.
- Do not replace retrieval, citation, document parsing, graph, or LLM packs;
  summarization consumes their handles and returns summary artifacts.
- Do not expose raw prompts, raw source documents, raw private spans, raw model
  outputs, raw provider payloads, credentials, or unbounded summaries in traces,
  audits, snapshots, SDK diagnostics, or examples.
- Do not silently summarize with a different provider, model, source subset, or
  lower-evidence mode when requested capability is unavailable.

## Ownership And Boundaries

- Pack id: `pack.knowledge.summarization.v1`.
- Family: `knowledge`.
- Backing service owner: summarization service provider.
- SDK surface: `sdk.packs.knowledge.summarization`.
- Command namespace: `summarization.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, optional model bridge
  composition, decorators, and sanitized diagnostics through approved
  composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `summarization.plan` | Build a summary job plan from sources, mode, strategy, budget, and evidence policy | Validates source access, chunking strategy, provider capability, redaction, and cost bounds |
| `summarization.validate_request` | Validate a summary request without generating output | Returns validation issues, provider capability diagnostics, and budget estimates |
| `summarization.summarize` | Produce extractive, abstractive, or hybrid summary from bounded sources | Requires source policy, mode support, evidence policy, resource limits, and quality metadata |
| `summarization.summarize_with_citations` | Produce summary with citation/evidence handles | Requires citation/evidence support or structured unavailable diagnostics |
| `summarization.summarize_many` | Summarize multiple documents/chunks and synthesize final summary | Requires chunk plan, partial summaries, synthesis strategy, and partial failure handling |
| `summarization.summarize_conversation` | Summarize dialogue, meeting, transcript, or event stream | Preserves speakers/turns as handles, action items when requested, and source evidence |
| `summarization.compress_context` | Compress context for agent/session continuation | Returns compression map, retained facts, dropped spans, unresolved items, and recovery notes |
| `summarization.refine_summary` | Update a prior summary with new evidence or constraints | Requires prior summary handle, delta sources, conflict handling, and version lineage |
| `summarization.compare_summaries` | Compare two or more summaries for coverage, contradiction, drift, or evidence gaps | Returns typed comparison report and bounded evidence |
| `summarization.evaluate_summary` | Evaluate summary quality against source and policy | Reports coverage, faithfulness, concision, sensitivity, freshness, and unsupported claims |
| `summarization.inspect_summary_evidence` | Inspect summary claims, source spans, citation handles, and compression map | Requires evidence permission and redaction |
| `summarization.inspect_provider` | Inspect provider modes, max input, context, citation support, streaming, quality metrics, and health | Returns sanitized capability metadata |

Every command must define typed command DTOs, typed success results, typed
partial/streaming results, validation results, typed denied/unavailable/
unsupported/conflict/quota/timeout/cancellation/failure results, redaction
profile, and replay metadata.

## DTO Model

Core DTOs:

- `SummarySource`: source handle, source kind, source span selectors, language,
  sensitivity class, freshness, citation handle, retrieval score, parsing
  status, and redaction profile.
- `SummaryRequest`: summary mode, target audience class, length policy, tone
  policy, output schema, language policy, evidence policy, quote policy,
  freshness policy, chunking strategy, synthesis strategy, resource budget, and
  quality thresholds.
- `SummaryPlan`: source inventory, chunk plan, map/reduce or rolling strategy,
  provider capability hash, estimated cost, estimated latency, risk flags,
  unavailable diagnostics, and replay pointer.
- `SummaryOutput`: summary handle, generated text handle, structured sections,
  extracted spans, claims, citations, evidence map, language, version hash,
  confidence, quality report handle, and redaction profile.
- `SummaryClaim`: claim handle, normalized claim text handle, source evidence
  handles, support status, confidence, freshness, contradiction flags, and
  redaction class.
- `SummaryEvidenceLink`: summary span, source span, citation handle, retrieval
  handle, graph provenance handle, extraction method, support score, and replay
  pointer.
- `CompressionMap`: original context handles, retained fact handles, compressed
  summary handle, dropped span hashes, unresolved item handles, decision
  handles, risk flags, and recovery notes handle.
- `SummaryComparisonReport`: compared summary handles, coverage matrix,
  contradiction set, drift score, missing evidence, duplicate claims, freshness
  gaps, and recommendations handle.
- `SummaryQualityReport`: coverage, faithfulness, concision, relevance,
  redundancy, sensitivity, unsupported-claim count, citation coverage, freshness
  score, and diagnostic codes.
- `SummaryProviderCapability`: modes, languages, max source size, max output
  size, streaming support, citation support, extractive span support,
  structured-output support, context-compression support, evaluation support,
  rate limits, lifecycle, and health.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `summarization.plan`
- `summarization.run`
- `summarization.citations`
- `summarization.context.compress`
- `summarization.conversation`
- `summarization.refine`
- `summarization.compare`
- `summarization.evaluate`
- `summarization.evidence.read`
- `summarization.provider.inspect`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id, and
  trace id when available.
- Source access is checked before planning and before generation. A summary may
  only use sources whose handles are policy-admissible for the application and
  session.
- Summary requests must declare mode, target length, output format, evidence
  policy, redaction profile, max input, max output, timeout, and budget.
- Citation/evidence summaries require citation or evidence support; absent
  support returns structured unavailable rather than silently producing
  uncited text.
- Context compression must preserve unresolved tasks, decisions, constraints,
  approvals, and recovery notes according to policy.
- Conversation summaries must treat speaker names, participants, messages,
  timestamps, and action items as potentially sensitive.
- Raw source documents, raw prompts, raw model outputs, raw provider payloads,
  and unbounded summaries are forbidden in observability.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
summary modes, supported source kinds, languages, output formats, structured
output support, citation support, extractive span support, context-compression
support, evaluation support, permission scopes, policy templates, resource
limits, approval rules, provider capability hashes, health, compatibility,
diagnostics, examples, redaction profiles, and documentation links.

The developer guide at `docs/developer-packs/knowledge/summarization.md` must
cover:

- manifest declaration and optional/required behavior
- source handles, source selectors, and source permission checks
- extractive, abstractive, hybrid, multi-document, conversation, meeting, and
  context-compression modes
- chunking, MapReduce, rolling summaries, synthesis strategies, and partial
  failure behavior
- command DTOs, result DTOs, streaming/pagination, timeout/cancellation, and
  structured errors
- evidence links, citation handles, claims, quality reports, comparison reports,
  compression maps, redaction, and replay pointers
- unavailable diagnostics, provider replacement, trace/audit interpretation,
  operational limits, and conformance tests

Examples must use generic synthetic sources. They must not bake in provider
names, application names, credentials, business workflows, private documents, or
domain-specific summary templates.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `summarization_pack_declared`
- `summarization_pack_admission_validated`
- `summarization_plan_created`
- `summarization_request_validated`
- `summary_generated`
- `summary_with_citations_generated`
- `summaries_synthesized`
- `conversation_summarized`
- `context_compressed`
- `summary_refined`
- `summaries_compared`
- `summary_evaluated`
- `summary_evidence_inspected`
- `summarization_provider_inspected`
- `summarization_pack_policy_decision`
- `summarization_pack_service_call_requested`
- `summarization_pack_service_call_succeeded`
- `summarization_pack_service_call_failed`
- `summarization_pack_unavailable`
- `summarization_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, supported
modes, source kind support, language support, citation/evidence support, context
compression support, evaluation support, command availability, provider health,
policy template hash, resource counters, bounded quality aggregates, and
sanitized replay pointers. Snapshots must exclude raw source documents, raw
prompts, raw model outputs, raw provider payloads, credentials, private
conversation text, and unbounded summaries.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: extractive, abstractive, hybrid, long-document, rolling,
  context-compression, evaluation, and unavailable behaviors are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  source-permission checks, evidence enforcement, and redaction wrap service
  calls.
- **Specification**: admission validates summary mode, source support,
  permission, provider capability, evidence policy, output format, budget, and
  compatibility.
- **Observer**: summary jobs, partial summaries, quality events, health, trace,
  and audit events are subscribable.
- **Memento**: summary version hashes, compression maps, quality reports,
  effective capability reports, and snapshots preserve recovery state.
- **Abstract Factory**: concrete provider adapters are created only by approved
  runtime-host composition roots.

## Risks And Mitigations

- Risk: summarization becomes a provider prompt pass-through. Mitigation: typed
  request/output/evidence DTOs, provider-neutral modes, and quality diagnostics.
- Risk: summaries hallucinate unsupported claims. Mitigation: evidence policy,
  claim support status, citation coverage, faithfulness score, and unsupported
  claim diagnostics.
- Risk: context compression loses critical task state. Mitigation: compression
  maps, retained facts, unresolved item handles, decision handles, and replay
  pointers.
- Risk: long-document summarization exceeds budgets. Mitigation: explicit
  chunking strategy, synthesis strategy, cost estimates, partial summaries,
  timeout/cancellation, and partial failure states.
- Risk: observability leaks private source text. Mitigation: source handles,
  redaction profiles, text handles, bounded snippets only when policy allows,
  and strict snapshot/audit exclusions.
