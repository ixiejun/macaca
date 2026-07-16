# Developer Code Pack Design

## Context

`pack.developer.code.v1` exposes code intelligence as a Macaca OS serviceized
capability. It lets applications analyze source code, inspect symbols, request
diagnostics, discover code actions, plan edits, generate patches, validate
patches, inspect diffs, estimate impact, and suggest tests without embedding
language-server, parser, scanner, editor, repository, terminal, or model-provider
logic into generic OS layers.

Code capability is host-sensitive. It can read proprietary source, modify files,
run analysis tools, leak secrets, or create patches that fail to apply. The pack
therefore uses typed workspace handles, policy gates, patch dry runs,
approval-required side effects, trace/audit records, bounded snapshots, and
provider replacement.

## Supplier Capability Matrix

| Supplier/standard | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Language Server Protocol | Diagnostics, symbols, definitions, references, formatting, semantic tokens, code actions, commands, workspace edits | Document/symbol/diagnostic/code-action/workspace-edit DTOs; language-provider capability; typed service calls |
| VS Code Extension API | Workspace, documents, diagnostics, code actions, commands, tasks, authentication, language features | Workspace handle, document handle, code action, edit plan, host capability permission, task/test suggestion hooks |
| Tree-sitter | Incremental syntax parsing and concrete syntax trees across languages | Syntax tree summary, parse diagnostics, language grammar capability, source range selectors |
| CodeQL and code scanning | Semantic analysis databases, queries, code scanning alerts, SARIF result exchange | Code scan request, finding DTO, SARIF-like result handle, severity/taxonomy, impact report |

The Macaca contract uses these as capability references only. Provider adapters
may translate to LSP, parser libraries, static analysis engines, model-assisted
edit planners, or scanners. The kernel, SDK, shells, and generic application
framework remain provider-neutral.

## Goals

- Provide stable pack id `pack.developer.code.v1` and command namespace
  `code.*`.
- Support workspace inventory, source indexing, parsing, symbol lookup,
  references, diagnostics, code actions, edit planning, patch generation, patch
  validation, patch application requests, rollback plans, diff inspection,
  impact analysis, test suggestion, code scan result import/inspection, and
  provider capability inspection.
- Preserve host safety with explicit workspace handles, read/write permissions,
  approval tokens, patch dry-runs, conflict diagnostics, redaction, and audit.
- Keep parser, LSP, scanner, model, repository, and terminal providers behind
  replaceable service providers.
- Require developer documentation at `docs/developer-packs/developer/code.md`.

## Non-Goals

- Do not implement concrete LSP clients, editor extensions, Tree-sitter parsers,
  CodeQL scanners, model providers, repository providers, terminal providers, or
  build systems in this proposal.
- Do not define application-specific coding workflows, PR workflows, issue
  workflows, repository policies, CI policies, or style guides.
- Do not expose raw source files, raw diffs, raw patches, credentials, raw scan
  payloads, raw provider payloads, prompts, manifests, package bytes, private
  keys, signatures, or unbounded analysis output in observability.
- Do not apply patches as a silent side effect. Patch application is a
  policy-checked, approval-aware request with dry-run and rollback metadata.

## Ownership And Boundaries

- Pack id: `pack.developer.code.v1`.
- Family: `developer`.
- Backing service owner: code intelligence service provider.
- SDK surface: `sdk.packs.developer.code`.
- Command namespace: `code.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, host capability bridges,
  decorators, and sanitized diagnostics through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `code.inspect_workspace` | Inspect workspace roots, languages, indexes, file counts, and capability state | Returns bounded inventory and denies out-of-scope paths |
| `code.index_workspace` | Build/update code index for symbols, diagnostics, references, or scan metadata | Requires resource budget and stale-index diagnostics |
| `code.parse_document` | Parse a source document and return syntax summary/diagnostics | Requires language capability and bounded tree summary |
| `code.find_symbols` | Find document/workspace symbols by name, kind, language, or range | Returns bounded symbol pages and confidence/source metadata |
| `code.find_references` | Find definitions, declarations, implementations, and references | Requires index/language capability and stale diagnostics |
| `code.get_diagnostics` | Return compiler/language/scanner diagnostics | Uses typed severity, ranges, codes, sources, and fix availability |
| `code.discover_code_actions` | Discover provider-supported actions for diagnostics/ranges | Returns typed code action descriptors, not provider commands |
| `code.plan_edit` | Plan an edit from intent, diagnostics, selected ranges, or code actions | Returns edit plan, affected files, risks, and required approvals |
| `code.generate_patch` | Generate a patch or workspace edit from an approved edit plan | Requires read permission, write intent, redaction, and dry-run metadata |
| `code.validate_patch` | Validate patch applicability, conflicts, formatting impact, and policy | Must not mutate files |
| `code.apply_patch_request` | Request applying a validated patch/workspace edit | Requires write permission, approval when needed, rollback plan, and audit |
| `code.inspect_diff` | Inspect diff hunks, changed symbols, generated files, and risk flags | Returns bounded hunk summaries and redacted snippets |
| `code.estimate_impact` | Estimate impacted symbols, files, tests, packages, and execution flows | Returns bounded impact report and confidence |
| `code.suggest_tests` | Suggest targeted tests and verification commands | Returns provider-neutral test suggestions and rationale handles |
| `code.import_scan_results` | Import SARIF-like/static analysis findings | Requires source mapping and redaction |
| `code.inspect_scan_findings` | Query bounded code-scan findings by severity, rule, path, symbol, or baseline | Returns typed findings and suppression metadata |
| `code.inspect_provider` | Inspect language, parser, LSP, scan, patch, diff, and test capability | Returns sanitized capability metadata |

Every command must define typed command DTOs, typed success results, typed
partial/paged results, validation results, typed denied/unavailable/unsupported/
conflict/quota/timeout/cancellation/failure results, redaction profile, and
replay metadata.

## DTO Model

Core DTOs:

- `CodeWorkspace`: workspace handle, declared roots, trust state, language
  inventory, index state, file count, excluded paths, policy scope, and health.
- `CodeDocument`: document handle, workspace handle, relative path handle,
  language id, version hash, content hash, size class, generated/vendor flags,
  and sensitivity class.
- `CodeRange`: document handle, line/column range, byte range hash, selector
  kind, and redaction class.
- `SyntaxTreeSummary`: language id, parser capability hash, root node kind,
  node counts, parse errors, changed ranges, and bounded structural summary.
- `CodeSymbol`: symbol handle, name hash or redacted name, kind, language,
  range, container, visibility, signature handle, definition/reference counts,
  and confidence.
- `CodeDiagnostic`: diagnostic handle, severity, source, rule/code, message
  handle, range, related ranges, fix availability, scan taxonomy, and baseline.
- `CodeAction`: action handle, title handle, kind, diagnostic handles,
  affected range, edit capability, command capability, and safety class.
- `WorkspaceEditPlan`: plan handle, intent handle, selected actions, affected
  documents, expected changes, risk flags, approvals, idempotency key, and
  rollback strategy.
- `CodePatch`: patch handle, patch format, affected documents, hunks, content
  hashes, generated-file markers, dry-run status, conflict diagnostics, and
  rollback handle.
- `CodeDiff`: base revision handle, target revision handle, file changes, hunk
  summaries, changed symbols, risk flags, binary/generated markers, and stats.
- `CodeImpactReport`: affected symbols, affected packages, dependency edges,
  execution flows, suggested tests, confidence, stale-index warnings, and
  bounded rationale handles.
- `CodeTestSuggestion`: command handle, test kind, scope, rationale handle,
  expected duration, required resources, and safety class.
- `CodeScanFinding`: finding handle, rule id, severity, location, message
  handle, taxonomy, related locations, baseline status, suppression status, and
  evidence handle.
- `CodeProviderCapability`: languages, parser support, LSP feature support,
  scan support, patch formats, diff support, formatting support, test discovery,
  max workspace size, rate limits, lifecycle, and health.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `code.workspace.read`
- `code.workspace.index`
- `code.document.read`
- `code.document.parse`
- `code.symbol.read`
- `code.diagnostic.read`
- `code.action.read`
- `code.edit.plan`
- `code.patch.generate`
- `code.patch.validate`
- `code.patch.apply`
- `code.diff.read`
- `code.impact.read`
- `code.test.suggest`
- `code.scan.import`
- `code.scan.read`
- `code.provider.inspect`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, workspace handle, and declared path scope when available.
- Reads are limited to declared workspace roots and denied for excluded paths,
  secrets, credentials, generated artifacts, vendor directories, or policy
  protected files unless explicitly permitted.
- Patch generation and application require separate permissions. Application
  requires a validated patch, current content hashes, conflict checks, approval
  when risky, and rollback metadata.
- Indexing, scanning, impact analysis, and test suggestion require resource
  budgets for time, memory, storage, file count, provider quota, output size, and
  snapshot retention.
- Raw source, raw patches, raw scan payloads, raw provider payloads, credentials,
  prompts, and unbounded diagnostics are forbidden in observability.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
language support, parser support, LSP feature support, scan support, patch
formats, diff support, permission scopes, policy templates, resource limits,
approval rules, provider capability hashes, health, compatibility, diagnostics,
examples, redaction profiles, and documentation links.

The developer guide at `docs/developer-packs/developer/code.md` must cover:

- manifest declaration and optional/required behavior
- workspace handles, path scopes, file sensitivity, and trust state
- code document, range, syntax, symbol, diagnostic, code action, edit plan,
  patch, diff, impact, test suggestion, and scan finding DTOs
- index freshness, language support, LSP-style features, parser support, and
  SARIF-like scan import
- patch dry-run, validation, approval, application request, rollback, and
  conflict diagnostics
- permissions, resource limits, unavailable diagnostics, provider replacement,
  trace/audit interpretation, and conformance tests

Examples must use synthetic workspace handles and small generic code snippets.
They must not bake in provider names, application names, credentials, business
workflows, private source code, or repository-specific conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `code_pack_declared`
- `code_pack_admission_validated`
- `code_workspace_inspected`
- `code_workspace_indexed`
- `code_document_parsed`
- `code_symbols_found`
- `code_references_found`
- `code_diagnostics_reported`
- `code_actions_discovered`
- `code_edit_planned`
- `code_patch_generated`
- `code_patch_validated`
- `code_patch_apply_requested`
- `code_diff_inspected`
- `code_impact_estimated`
- `code_tests_suggested`
- `code_scan_results_imported`
- `code_scan_findings_inspected`
- `code_provider_inspected`
- `code_pack_policy_decision`
- `code_pack_service_call_requested`
- `code_pack_service_call_succeeded`
- `code_pack_service_call_failed`
- `code_pack_unavailable`
- `code_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, language
support, index state hashes, parser capability hashes, scan capability hashes,
patch format support, command availability, provider health, policy template
hash, resource counters, bounded workspace statistics, and sanitized replay
pointers. Snapshots must exclude raw source, raw patches, raw diffs, raw scan
payloads, credentials, prompts, raw provider payloads, and unbounded diagnostics.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: parser, language server, scanner, edit planner, patch validator,
  impact analyzer, test suggester, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering, path
  scope, secret redaction, and patch safety checks wrap service calls.
- **Specification**: admission validates workspace scope, language support,
  command availability, permission, provider capability, patch preconditions,
  and compatibility.
- **Observer**: index updates, diagnostics, patch requests, scan findings,
  health, trace, and audit events are subscribable.
- **Memento**: index hashes, document version hashes, edit plans, patch dry-run
  records, rollback plans, snapshots, and replay pointers preserve recovery
  state.
- **Abstract Factory**: concrete provider adapters are created only by approved
  runtime-host composition roots.

## Risks And Mitigations

- Risk: code pack becomes a shell-owned editor workflow. Mitigation: shells only
  render diagnostics/actions/diffs and submit typed service commands.
- Risk: patch application mutates files unsafely. Mitigation: separate generate,
  validate, and apply-request commands with content hashes, approval, and
  rollback metadata.
- Risk: source leaks through logs. Mitigation: handles/hashes, redaction
  profiles, bounded snippets only under policy, and snapshot/audit exclusions.
- Risk: stale indexes produce wrong impact reports. Mitigation: index state,
  version hashes, stale warnings, confidence, and replayable source inventory.
- Risk: provider-specific features leak into portable app logic. Mitigation:
  descriptors, capability hashes, Strategy adapters, and boundary gates.
