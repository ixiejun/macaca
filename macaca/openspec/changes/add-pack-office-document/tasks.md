## 1. Supplier API Research And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, OpenSpec rules, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Study Google Docs API for document structure, tabs/body/content, atomic `documents.batchUpdate`, styles, tables, lists, inline objects, and revision behavior.
- [x] 1.3 Study Microsoft Word JavaScript API for document object access, ranges, content controls, comments, styles, and tracked-change APIs.
- [x] 1.4 Study OpenXML / WordprocessingML for packages, paragraphs, runs, tables, styles, comments, revisions, and strongly typed document structures.
- [x] 1.5 Study LibreOffice UNO Writer text document APIs for paragraphs, text ranges, styles, fields, tables, and automation.
- [x] 1.6 Produce a supplier capability comparison memo mapping Google Docs, Word JS, OpenXML, and LibreOffice UNO concepts into Macaca provider-neutral document DTOs and commands.
- [x] 1.7 Define explicit non-goals for concrete Word, Google Docs, OpenXML, LibreOffice, PDF, cloud-drive, OCR, conversion providers, legal/report/template workflows, raw provider pass-through, and provider-specific routing.
- [x] 1.8 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.office.document.v1` descriptor metadata: pack id, family, lifecycle, stability, create/open/import support, structure support, range support, style support, table/list support, comment support, revision support, export support, collaboration event support, formats, auth modes, command schemas, permission scopes, policy templates, resource budgets, approval requirements, data-governance class, SDK metadata, documentation link, compatibility, and diagnostics.
- [x] 2.2 Define provider-neutral DTOs for `DocumentScope`, `DocumentProviderCapability`, `DocumentHandle`, `DocumentStructure`, `DocumentRange`, `DocumentParagraph`, `DocumentRun`, `DocumentTable`, `DocumentStyle`, `DocumentComment`, `DocumentRevision`, `DocumentEditOperation`, `DocumentEditPlan`, `DocumentExportPlan`, `DocumentArtifactHandle`, and `DocumentCollaborationEvent`.
- [x] 2.3 Define typed command/result DTOs for `document.inspect_provider`, `document.create_document_request`, `document.import_document_request`, `document.open_document`, `document.inspect_structure`, `document.read_range`, `document.inspect_styles`, `document.inspect_comments`, `document.inspect_revisions`, `document.plan_edit`, `document.edit_request`, `document.comment_request`, `document.redline_request`, `document.plan_revision_resolution`, `document.revision_resolution_request`, `document.plan_export`, `document.export_request`, `document.inspect_events`, and `document.get_artifact_handle`.
- [x] 2.4 Define typed success, paged, partial, denied, unavailable, unsupported, conflict, stale-version, schema-mismatch, format-unsupported, export-denied, write-denied, revision-unsupported, quota, timeout, cancellation, approval-required, and failure DTOs.
- [x] 2.5 Define stable descriptor hashing, provider capability hashing, document format hashing, document version hashing, range anchor hashing, structure projection hashing, style catalog hashing, edit plan hashing, export plan hashing, artifact handle hashing, event cursor hashing, and redaction metadata.
- [x] 2.6 Add descriptor and DTO compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, schema evolution, document handles, structures, ranges, paragraphs, runs, tables, styles, comments, revisions, edit plans, export plans, artifacts, event cursors, redaction profiles, and serde compatibility.

## 3. Admission, Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement manifest declaration validation for required and optional `pack.office.document.v1` declarations.
- [x] 3.2 Implement permission validation for `document.provider.inspect`, `document.create`, `document.import`, `document.open`, `document.structure.read`, `document.range.read`, `document.style.read`, `document.comment.read`, `document.comment.write`, `document.revision.read`, `document.revision.write`, `document.edit`, `document.export`, `document.events.read`, and `document.artifact.read`.
- [x] 3.3 Implement provider/document/range/comment/revision/artifact/event scope checks for declared documents, private documents, personal data, embedded media, comments, tracked changes, denied scopes, and stale handles.
- [x] 3.4 Implement policy checks for format compatibility, version preconditions, range anchors, batch edit validation, style/table/list compatibility, comment visibility, revision support, export format policy, artifact retention, collaboration notification policy, and output redaction.
- [x] 3.5 Implement resource reservation for document size, range text size, structure depth, paragraph/table/list count, comment count, revision count, edit operation count, export size, artifact size, provider quota, network transfer, timeout, memory, storage, streaming output, and retained snapshots.
- [x] 3.6 Implement entitlement checks and structured unavailable/denied diagnostics for missing provider, disabled pack, missing credential reference, missing document permission, unsupported format, unsupported comment/revision/export support, disabled network, missing entitlement, provider quota, and host resource denial.
- [x] 3.7 Implement approval behavior for private documents, contracts, personal data, comments, revisions, embedded media, collaborator-visible edits, destructive edits, exports, revision accept/reject, and operations that notify collaborators or external systems.
- [x] 3.8 Add tests proving denied, validation, quota, unavailable, conflict, stale-version, schema-mismatch, format-unsupported, export-denied, write-denied, revision-unsupported, unsupported, timeout, cancellation, and approval-required paths do not call concrete providers, mutate documents, comment, redline, export, notify collaborators, or expose raw document data.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Implement or bind the office document service provider behind the service runtime; do not construct document providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add a deterministic unavailable provider that returns typed unavailable/unsupported diagnostics and complete discovery metadata.
- [x] 4.3 Add mock provider support for provider inspection, create/import/open, structure inspection, range reading, style/comment/revision inspection, edit planning/request, comment/redline requests, revision resolution planning/request, export planning/request, event inspection, artifact handles, health, and provider capability inspection.
- [x] 4.4 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, bounded paging, range anchor freshness, stale-version diagnostics, schema/format mismatch diagnostics, artifact retention, and rate-limit diagnostics.
- [x] 4.5 Add Strategy implementations for provider adapters, format readers, range resolvers, edit validators, comment/revision strategies, export renderers, artifact providers, event readers, redaction, and unavailable behavior.
- [x] 4.6 Add side-effect safety support for idempotency keys, provider state validation, document version preconditions, range anchor freshness, format compatibility checks, approval state, collaborator notification policy, artifact retention, and non-mutating plan commands.
- [x] 4.7 Add provider capability reporting for available, degraded, preview, unavailable, unsupported, retired, format-limited, structure-limited, comment-limited, revision-limited, export-limited, collaboration-limited, network-limited, and quota-limited states.

## 5. SDK, WASM ABI, Application Framework, And Examples

- [x] 5.1 Extend SDK discovery for `pack.office.document.v1` with command schemas, format support, structure support, range support, style support, comment support, revision support, export support, collaboration event support, examples, availability, diagnostics, documentation link, provider class, capability hash, compatibility, and redaction profile.
- [x] 5.2 Extend application admission so required declarations block readiness when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders for all `document.*` commands; helpers must only build canonical traced service calls and must never construct document clients, access credentials, call provider APIs, mutate documents, comment, redline, export raw documents, read private comments, or bypass policy.
- [x] 5.4 Extend WASM/app ABI descriptors so applications can discover document commands, declare permissions, receive unavailable diagnostics, and submit typed service calls through the canonical execution path.
- [x] 5.5 Add generic app-facing examples for provider inspection, document creation/import/opening, structure inspection, range reading, style/comment/revision inspection, edit planning/request, comment/redline requests, revision resolution, export planning/request, event inspection, and artifact handles.
- [x] 5.6 Add unavailable-provider, missing-document-permission, stale-version, range-anchor-stale, format-unsupported, schema-mismatch, export-denied, write-approval, revision-unsupported, provider-quota, network-denied, comment-redacted, and artifact-denied examples that demonstrate diagnostics without provider names, credentials, private comments, personal data, full document text, raw exports, or workflow-specific conventions.

## 6. Trace, Audit, Replay, Security, And Gates

- [x] 6.1 Emit sanitized declaration, admission, provider-inspection, create/import/open, structure-inspection, range-read, style-inspection, comment-inspection, revision-inspection, edit-plan, edit-request, comment-request, redline-request, revision-resolution-plan, revision-resolution-request, export-plan, export-request, event-inspection, artifact-handle, policy, entitlement, resource, approval, health, snapshot, unavailable, and failure events.
- [x] 6.2 Ensure traces, audits, snapshots, SDK diagnostics, and examples exclude raw credentials, tokens, private comments, personal data, raw full document text, raw embedded media, raw exports, raw provider payloads, prompts, manifests, package bytes, private keys, signatures, and unbounded document trees.
- [x] 6.3 Add replay tests proving every `document.*` command is trace-addressable through the canonical service path and that snapshots contain enough bounded metadata for recovery diagnostics.
- [x] 6.4 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete Word, Google Docs, OpenXML, LibreOffice, PDF, cloud-drive, OCR, conversion, credential-manager, artifact-provider, or provider adapters.
- [x] 6.5 Add no-direct-provider-call gates proving SDK helpers, WASM ABI handlers, app admission, web, CLI, and frontend paths route through descriptor-owned service commands.
- [x] 6.6 Add boundary tests proving optional provider absence returns structured unavailable diagnostics and never crashes, hangs, silently falls back, mutates documents, comments, redlines, exports, accepts/rejects revisions, notifies collaborators, retrieves raw document data, contacts providers, or fakes success.
- [x] 6.7 Run `openspec validate add-pack-office-document --strict`, targeted cargo tests, boundary gates, file-size gates, and audit replay checks before marking implementation complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/office/document.md` with purpose, capability model, manifest declaration, required versus optional behavior, permissions, provider scopes, document handles, formats, structures, sections, paragraphs, runs, tables, lists, ranges, styles, comments, revisions, edit plans, export plans, artifacts, events, unavailable diagnostics, provider replacement, and operational limits.
- [x] 7.2 Document all command DTOs and result DTOs with field-level explanations, idempotency semantics, redaction behavior, pagination/streaming behavior, timeout/cancellation behavior, plan/request behavior, approval behavior, artifact retention behavior, version preconditions, format compatibility, and structured error codes.
- [x] 7.3 Document supplier/API mapping: Google Docs API, Microsoft Word JavaScript API, OpenXML/WordprocessingML, and LibreOffice UNO concepts mapped to Macaca abstractions, including what is intentionally not exposed as OS semantics.
- [x] 7.4 Add generic examples for provider inspection, document create/import/open, structure inspection, range read, edit plan/request, comments, redlines, revision resolution, export, events, artifact handles, and unavailable diagnostics using synthetic document data only.
- [x] 7.5 Add conformance checklist and test guidance for provider authors: descriptor completeness, document/range scope validation, format compatibility, edit validation, version conflicts, comment/revision safety, export validation, artifact redaction, event redaction, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and redaction.
- [x] 7.6 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-office-document` complete.
