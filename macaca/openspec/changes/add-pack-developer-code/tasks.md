## 1. Supplier API Research And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, OpenSpec rules, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Study Language Server Protocol 3.17 for diagnostics, symbols, references, semantic tokens, code actions, commands, formatting, and workspace edits.
- [x] 1.3 Study VS Code Extension API for workspace/document handles, diagnostics, code actions, commands, tasks, authentication, language features, and workspace edits.
- [x] 1.4 Study Tree-sitter documentation for incremental parsing, syntax trees, changed ranges, grammar support, and parser lifecycle.
- [x] 1.5 Study GitHub CodeQL and SARIF documentation for semantic scan databases, code scanning alerts, severity, taxonomy, related locations, and scan result exchange.
- [x] 1.6 Produce a supplier capability comparison memo mapping LSP, VS Code API, Tree-sitter, CodeQL, and SARIF concepts into Macaca provider-neutral DTOs and commands.
- [x] 1.7 Define explicit non-goals for concrete editor integrations, parser engines, model clients, repository workflows, terminal/build execution, and application-specific coding flows.
- [x] 1.8 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.developer.code.v1` descriptor metadata: pack id, family, lifecycle, stability, language support, parser support, LSP-style feature support, scan support, patch support, command schemas, permission scopes, policy templates, resource budgets, approval requirements, data-governance class, SDK metadata, documentation link, compatibility, and diagnostics.
- [x] 2.2 Define provider-neutral DTOs for `CodeWorkspace`, `CodeDocument`, `CodeRange`, `SyntaxTreeSummary`, `CodeSymbol`, `CodeDiagnostic`, `CodeAction`, `WorkspaceEditPlan`, `CodePatch`, `CodeDiff`, `CodeImpactReport`, `CodeTestSuggestion`, `CodeScanFinding`, and `CodeProviderCapability`.
- [x] 2.3 Define typed command/result DTOs for `code.inspect_workspace`, `code.index_workspace`, `code.parse_document`, `code.find_symbols`, `code.find_references`, `code.get_diagnostics`, `code.discover_code_actions`, `code.plan_edit`, `code.generate_patch`, `code.validate_patch`, `code.apply_patch_request`, `code.inspect_diff`, `code.estimate_impact`, `code.suggest_tests`, `code.import_scan_results`, `code.inspect_scan_findings`, and `code.inspect_provider`.
- [x] 2.4 Define typed success, paged result, partial result, dry-run result, validation issue, denied, unavailable, unsupported, conflict, quota, timeout, cancellation, approval-required, and failure DTOs.
- [x] 2.5 Define stable descriptor hashing, provider capability hashing, workspace inventory hashing, document version hashing, index state hashing, patch content hashing, diff hashing, scan baseline hashing, and redaction metadata.
- [x] 2.6 Add descriptor and DTO compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, schema evolution, workspace scopes, patch formats, scan findings, redaction profiles, and serde compatibility.

## 3. Admission, Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement manifest declaration validation for required and optional `pack.developer.code.v1` declarations.
- [x] 3.2 Implement permission validation for `code.workspace.read`, `code.workspace.index`, `code.document.read`, `code.document.parse`, `code.symbol.read`, `code.diagnostic.read`, `code.action.read`, `code.edit.plan`, `code.patch.generate`, `code.patch.validate`, `code.patch.apply`, `code.diff.read`, `code.impact.read`, `code.test.suggest`, `code.scan.import`, `code.scan.read`, and `code.provider.inspect`.
- [ ] 3.3 Implement workspace/path scope checks for declared roots, excluded paths, secret files, credentials, generated artifacts, vendor directories, binary files, and protected files.
- [ ] 3.4 Implement policy checks for language support, parser support, index freshness, scan import source mapping, code-action safety, edit-plan risk, patch format, current content hashes, rollback metadata, and output redaction.
- [ ] 3.5 Implement resource reservation for file count, source bytes, syntax tree size, index size, scan finding count, diff size, patch size, timeout, memory, storage, provider quota, streaming output, and retained snapshots.
- [ ] 3.6 Implement entitlement checks and structured unavailable/denied diagnostics for missing provider, disabled pack, missing workspace trust, missing path permission, unsupported language, stale index, unsupported patch format, missing scanner, missing parser, missing entitlement, and host resource denial.
- [ ] 3.7 Implement approval behavior for patch application, edits touching protected files, generated-file writes, broad workspace rewrites, destructive deletes, secret-adjacent files, and long-running/high-cost analysis.
- [ ] 3.8 Add tests proving denied, validation, quota, unavailable, conflict, stale-index, and approval-required paths do not call concrete providers or mutate files.

## 4. Service Provider And Runtime Integration

- [ ] 4.1 Implement or bind the code intelligence service provider behind the service runtime; do not construct code providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add a deterministic unavailable provider that returns typed unavailable/unsupported diagnostics and complete discovery metadata.
- [ ] 4.3 Add mock provider support for workspace inspection, indexing, parsing, symbols, references, diagnostics, code actions, edit planning, patch generation, patch validation, apply requests, diff inspection, impact reports, test suggestions, scan imports, scan finding inspection, and provider capability inspection.
- [ ] 4.4 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, bounded streaming, paged results, and stale-index diagnostics.
- [ ] 4.5 Add Strategy implementations for parser adapters, LSP-style feature adapters, scan result adapters, edit planners, patch validators, impact analyzers, test suggesters, and unavailable behavior.
- [ ] 4.6 Add patch safety support for dry-run, content-hash verification, conflict detection, generated/binary file checks, approval state, rollback planning, and non-mutating validation.
- [ ] 4.7 Add provider capability reporting for available, degraded, preview, unavailable, unsupported, retired, language-specific, feature-specific, index-stale, scan-limited, patch-limited, and quota-limited states.

## 5. SDK, WASM ABI, Application Framework, And Examples

- [x] 5.1 Extend SDK discovery for `pack.developer.code.v1` with command schemas, languages, parser support, LSP-style features, scan support, patch formats, examples, availability, diagnostics, documentation link, provider class, capability hash, compatibility, and redaction profile.
- [x] 5.2 Extend application admission so required declarations block readiness when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders for all `code.*` commands; helpers must only build canonical traced service calls and must never construct parsers, language servers, scanners, model clients, repository clients, terminal commands, or providers.
- [ ] 5.4 Extend WASM/app ABI descriptors so applications can discover code commands, declare permissions, receive unavailable diagnostics, and submit typed service calls through the canonical execution path.
- [x] 5.5 Add generic app-facing examples for inspecting a workspace, parsing a document, finding symbols, reading diagnostics, discovering code actions, planning an edit, generating a patch, validating a patch, requesting patch application, inspecting a diff, estimating impact, suggesting tests, and importing scan results.
- [x] 5.6 Add unavailable-provider, missing-workspace-permission, unsupported-language, stale-index, patch-conflict, and approval-required examples that demonstrate diagnostics without provider names, credentials, private source code, repository-specific conventions, or application workflows.

## 6. Trace, Audit, Replay, Security, And Gates

- [ ] 6.1 Emit sanitized declaration, admission, workspace, indexing, parsing, symbol, reference, diagnostic, code-action, edit-plan, patch, diff, impact, test-suggestion, scan, provider-inspection, policy, entitlement, resource, approval, health, snapshot, unavailable, and failure events.
- [ ] 6.2 Ensure traces, audits, snapshots, SDK diagnostics, and examples exclude raw source files, raw patches, raw diffs, raw scan payloads, raw provider payloads, credentials, prompts, manifests, package bytes, private keys, signatures, and unbounded diagnostics.
- [ ] 6.3 Add replay tests proving every `code.*` command is trace-addressable through the canonical service path and that snapshots contain enough bounded metadata for recovery diagnostics.
- [ ] 6.4 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete language servers, editor APIs, parser libraries, scanner engines, repository clients, terminal clients, model clients, or provider adapters.
- [ ] 6.5 Add no-direct-provider-call gates proving SDK helpers, WASM ABI handlers, app admission, web, CLI, and frontend paths route through descriptor-owned service commands.
- [ ] 6.6 Add boundary tests proving optional provider absence returns structured unavailable diagnostics and never crashes, hangs, silently falls back, mutates files, or fakes success.
- [ ] 6.7 Run `openspec validate add-pack-developer-code --strict`, targeted cargo tests, boundary gates, file-size gates, and audit replay checks before marking implementation complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/developer/code.md` with purpose, capability model, manifest declaration, required versus optional behavior, permissions, workspace handles, path scopes, documents, ranges, syntax trees, symbols, diagnostics, code actions, edit plans, patches, diffs, impact reports, test suggestions, scan findings, unavailable diagnostics, provider replacement, and operational limits.
- [x] 7.2 Document all command DTOs and result DTOs with field-level explanations, idempotency semantics, redaction behavior, pagination/streaming behavior, timeout/cancellation behavior, dry-run behavior, approval behavior, rollback behavior, and structured error codes.
- [x] 7.3 Document supplier/API mapping: LSP, VS Code API, Tree-sitter, CodeQL, and SARIF concepts mapped to Macaca abstractions, including what is intentionally not exposed as OS semantics.
- [x] 7.4 Add generic examples for analysis, symbols, diagnostics, code action discovery, edit planning, patch generation, patch validation, patch apply request, diff inspection, impact analysis, test suggestion, and scan finding inspection using synthetic source only.
- [x] 7.5 Add conformance checklist and test guidance for provider authors: descriptor completeness, language support, parser support, index freshness, diagnostics, code actions, patch safety, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and redaction.
- [x] 7.6 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-developer-code` complete.
