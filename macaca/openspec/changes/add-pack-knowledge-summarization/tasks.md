## 1. Supplier API Research And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, OpenSpec rules, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Study OpenAI official docs for model-based summarization, Responses API behavior, prompt guidance, structured output constraints, streaming, and safety considerations.
- [x] 1.3 Study Anthropic context engineering guidance for context summarization, compaction, preserving decisions, unresolved work, and recovery-critical state.
- [x] 1.4 Study Azure AI Language summarization docs for extractive versus abstractive summarization, input limits, action semantics, and result metadata.
- [x] 1.5 Study Google Vertex AI summarization samples and long-document patterns for stuffing, chunking, MapReduce, overlapping chunks, and rolling summaries.
- [x] 1.6 Study Amazon Comprehend docs for entity, key phrase, event, sentiment, and topic insights that can back extractive or hybrid summary signals.
- [x] 1.7 Produce a supplier capability comparison memo mapping modes, source limits, evidence support, output structure, quality metadata, streaming, long-document strategy, and unavailable behavior into Macaca provider-neutral abstractions.
- [x] 1.8 Define explicit non-goals for concrete provider adapters, application-specific summary templates, provider prompt pass-through, and silent fallback.
- [x] 1.9 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.knowledge.summarization.v1` descriptor metadata: pack id, family, lifecycle, stability, supported modes, supported source kinds, command schemas, permission scopes, policy templates, resource budgets, approval requirements, data-governance class, SDK metadata, documentation link, compatibility, and diagnostics.
- [x] 2.2 Define provider-neutral DTOs for `SummarySource`, `SummaryRequest`, `SummaryPlan`, `SummaryOutput`, `SummaryClaim`, `SummaryEvidenceLink`, `CompressionMap`, `SummaryComparisonReport`, `SummaryQualityReport`, and `SummaryProviderCapability`.
- [x] 2.3 Define typed command/result DTOs for `summarization.plan`, `summarization.validate_request`, `summarization.summarize`, `summarization.summarize_with_citations`, `summarization.summarize_many`, `summarization.summarize_conversation`, `summarization.compress_context`, `summarization.refine_summary`, `summarization.compare_summaries`, `summarization.evaluate_summary`, `summarization.inspect_summary_evidence`, and `summarization.inspect_provider`.
- [x] 2.4 Define typed success, streaming, paged, partial, validation issue, denied, unavailable, unsupported, conflict, quota, timeout, cancellation, and failure DTOs.
- [x] 2.5 Define stable descriptor hashing, provider capability hashing, summary version hashing, source inventory hashing, compression-map hashing, evidence-map hashing, and redaction metadata.
- [x] 2.6 Add descriptor and DTO compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, schema evolution, evidence maps, compression maps, quality reports, redaction profiles, and serde compatibility.

## 3. Admission, Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement manifest declaration validation for required and optional `pack.knowledge.summarization.v1` declarations.
- [x] 3.2 Implement permission validation for `summarization.plan`, `summarization.run`, `summarization.citations`, `summarization.context.compress`, `summarization.conversation`, `summarization.refine`, `summarization.compare`, `summarization.evaluate`, `summarization.evidence.read`, and `summarization.provider.inspect`.
- [x] 3.3 Implement source access checks before planning and generation for document, retrieval, citation, graph, message, transcript, and prior-summary source handles.
- [x] 3.4 Implement policy checks for summary mode, output schema, target length, language policy, evidence policy, quote policy, freshness, sensitivity, context-compression retention, and quality thresholds.
- [x] 3.5 Implement resource reservation for max input tokens/bytes, max output tokens/bytes, chunk count, partial summary count, streaming output, timeout, memory, storage, network, provider quota, evaluation cost, and retained snapshots.
- [x] 3.6 Implement entitlement checks and structured unavailable/denied diagnostics for missing provider, disabled pack, missing source permission, missing citation/evidence support, missing context-compression support, missing evaluation support, missing entitlement, unsupported language, unsupported source kind, and host resource denial.
- [x] 3.7 Implement approval behavior for summaries that disclose sensitive source text, export summaries outside the application boundary, compress irreversible session state, or run long/expensive summarization jobs.
- [x] 3.8 Add tests proving denied, validation, quota, unavailable, and approval-required paths do not call concrete providers.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Implement or bind the summarization service provider behind the service runtime; do not construct summarization providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add a deterministic unavailable provider that returns typed unavailable/unsupported diagnostics and complete discovery metadata.
- [x] 4.3 Add mock provider support for extractive, abstractive, hybrid, multi-document, conversation, context-compression, refinement, comparison, evaluation, evidence inspection, and provider capability inspection commands.
- [x] 4.4 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, bounded streaming, and paginated/partial result behavior.
- [x] 4.5 Add Strategy implementations for extractive summarization, abstractive summarization, hybrid summarization, long-document synthesis, rolling summaries, context compression, summary evaluation, and unavailable behavior.
- [x] 4.6 Add long-document execution support for chunk plans, map summaries, reduce/synthesis summaries, overlap policies, partial failures, and resumable checkpoints.
- [x] 4.7 Add evidence and citation integration hooks through declared pack/service handles, returning structured unavailable diagnostics when evidence/citation capability is absent.
- [x] 4.8 Add provider capability reporting for available, degraded, preview, unavailable, unsupported, retired, mode-specific, language-specific, source-specific, streaming-specific, and quota-limited states.

## 5. SDK, WASM ABI, Application Framework, And Examples

- [x] 5.1 Extend SDK discovery for `pack.knowledge.summarization.v1` with command schemas, summary modes, source kinds, languages, output formats, citation support, context-compression support, evaluation support, examples, availability, diagnostics, documentation link, provider class, capability hash, compatibility, and redaction profile.
- [x] 5.2 Extend application admission so required declarations block readiness when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders for all `summarization.*` commands; helpers must only build canonical traced service calls and must never construct providers, prompts, or model clients.
- [x] 5.4 Extend WASM/app ABI descriptors so applications can discover summarization commands, declare permissions, receive unavailable diagnostics, and submit typed service calls through the canonical execution path.
- [x] 5.5 Add generic app-facing examples for planning a summary, producing an extractive summary, producing an abstractive summary, summarizing many sources, summarizing a conversation, compressing context, comparing summaries, evaluating summary quality, and inspecting evidence.
- [x] 5.6 Add unavailable-provider, missing-source-permission, missing-citation-support, and quota-denied examples that demonstrate diagnostics without provider names, credentials, application-specific workflows, private documents, or domain-specific summary templates.

## 6. Trace, Audit, Replay, Security, And Gates

- [x] 6.1 Emit sanitized declaration, admission, planning, validation, policy, entitlement, resource, approval, service-call, summary-generation, conversation-summary, context-compression, refinement, comparison, evaluation, evidence-inspection, health, snapshot, unavailable, and failure events.
- [x] 6.2 Ensure traces, audits, snapshots, SDK diagnostics, and examples exclude raw credentials, raw source documents, raw prompts, raw model outputs, raw provider payloads, raw private spans, private conversation text, unbounded summaries, package bytes, manifests, private keys, and signatures.
- [x] 6.3 Add replay tests proving every `summarization.*` command is trace-addressable through the canonical service path and that snapshots contain enough bounded metadata for recovery diagnostics.
- [x] 6.4 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete summarization providers, model clients, prompt templates, cloud SDKs, or provider-specific adapters.
- [x] 6.5 Add no-direct-provider-call gates proving SDK helpers, WASM ABI handlers, app admission, web, CLI, and frontend paths route through descriptor-owned service commands.
- [x] 6.6 Add boundary tests proving optional provider absence returns structured unavailable diagnostics and never crashes, hangs, silently falls back, or fakes success.
- [x] 6.7 Run `openspec validate add-pack-knowledge-summarization --strict`, targeted cargo tests, boundary gates, file-size gates, and audit replay checks before marking implementation complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/knowledge/summarization.md` with purpose, capability model, manifest declaration, required versus optional behavior, permissions, source handles, extractive/abstractive/hybrid modes, multi-document synthesis, conversation summaries, context compression, evidence links, claims, quality reports, comparison reports, unavailable diagnostics, provider replacement, and operational limits.
- [x] 7.2 Document all command DTOs and result DTOs with field-level explanations, summary mode semantics, idempotency semantics, redaction behavior, streaming/pagination behavior, timeout/cancellation behavior, partial failure behavior, and structured error codes.
- [x] 7.3 Document supplier/API mapping: OpenAI, Anthropic, Azure AI Language, Google Vertex AI, and Amazon Comprehend concepts mapped to Macaca abstractions, including what is intentionally not exposed as OS semantics.
- [x] 7.4 Add generic examples for extractive summary, abstractive summary, cited summary, multi-document synthesis, conversation summary, context compression, comparison, evaluation, and evidence inspection using synthetic data only.
- [x] 7.5 Add conformance checklist and test guidance for provider authors: descriptor completeness, mode support, source access, evidence policy, quality diagnostics, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and redaction.
- [x] 7.6 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-knowledge-summarization` complete.
