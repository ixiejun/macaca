# Developer Code Pack

`pack.developer.code.v1` provides provider-neutral workspace inspection,
indexing, parsing, symbol search, diagnostics, code actions, edit planning,
patch generation, diff inspection, impact estimation, test suggestion, scan
import, scan finding inspection, and provider capability discovery.

The pack is descriptor-only until a serviceized code provider is registered by
the runtime composition root. Applications hold workspace, document, range,
patch, diff, scan, and impact references; they do not receive raw source,
patches, diffs, prompts, credentials, or provider payloads through traces.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.developer.code.v1"]
```

Unavailable optional declarations report `developer_code_provider_not_installed`.
Required declarations block readiness until a descriptor-compatible provider,
policy, resource, entitlement, approval, trace, audit, and redaction chain is
available.

## Permission Scopes

- `code.workspace.read`, `code.workspace.index`, `code.document.read`,
  `code.document.parse`, `code.symbol.read`, and `code.diagnostic.read`.
- `code.action.read`, `code.edit.plan`, `code.patch.generate`,
  `code.patch.validate`, `code.patch.apply`, and `code.diff.read`.
- `code.impact.read`, `code.test.suggest`, `code.scan.import`,
  `code.scan.read`, and `code.provider.inspect`.

## Commands

- `code.inspect_workspace`, `code.index_workspace`, and
  `code.parse_document`.
- `code.find_symbols`, `code.find_references`, `code.get_diagnostics`, and
  `code.discover_code_actions`.
- `code.plan_edit`, `code.generate_patch`, `code.validate_patch`, and
  `code.apply_patch_request`.
- `code.inspect_diff`, `code.estimate_impact`, `code.suggest_tests`,
  `code.import_scan_results`, `code.inspect_scan_findings`, and
  `code.inspect_provider`.

## DTOs And Results

Core DTOs include `CodeWorkspace`, `CodeDocument`, `CodeRange`,
`SyntaxTreeSummary`, `CodeSymbol`, `CodeDiagnostic`, `CodeAction`,
`WorkspaceEditPlan`, `CodePatch`, `CodeDiff`, `CodeImpactReport`,
`CodeTestSuggestion`, `CodeScanFinding`, and `CodeProviderCapability`.
Result statuses cover success, paging, partial results, dry runs, denied,
unavailable, unsupported, conflict, quota, timeout, cancellation, approval
required, validation issues, and provider failure.

## Command DTO Details

Every command wrapper carries a `DeveloperCommandEnvelope`:

- `subject_ref`: workspace, document, patch, diff, scan, or provider subject.
- `parameters`: reference-only arguments such as `workspace_ref`,
  `document_ref`, `range_ref`, `patch_ref`, `diff_ref`, `scan_ref`,
  `content_hash`, and `approval_ref`.
- `cursor` and `page_size`: bounded pagination for symbols, references,
  diagnostics, code actions, scan findings, and impact records.
- `idempotency_key`: stable key for indexing, planning, patch validation,
  patch apply request, and scan import commands.

Result envelopes return `status`, optional `data`, optional paged data, and a
trace-safe error. Dry-run commands return plan, patch, diff, or validation refs;
approval-required results include an approval ref without mutating source.
Rollback behavior is represented by `WorkspaceEditPlan.rollback_ref`.

## Supplier/API Mapping

- LSP diagnostics, document symbols, workspace symbols, references, code
  actions, and workspace edits map to document, range, symbol, diagnostic,
  action, and edit-plan DTOs.
- VS Code workspace/document handles map to `CodeWorkspace` and
  `CodeDocument`; extension commands are not exposed as OS semantics.
- Tree-sitter syntax trees map to `SyntaxTreeSummary`; raw parse trees and
  source text remain provider-private.
- CodeQL databases and SARIF alerts map to scan finding refs, rule refs,
  severities, ranges, and baseline state.
- Provider-specific language servers, parser lifecycles, editor UI commands,
  model prompts, and repository workflows stay behind adapters.

## Examples

Inspect a workspace with synthetic references:

```json
{
  "subject_ref": "workspace:demo",
  "parameters": { "root_scope_ref": "scope:source" },
  "idempotency_key": "workspace-demo-inspect"
}
```

Plan and validate a patch without mutating files:

```json
{
  "subject_ref": "workspace:demo",
  "parameters": {
    "edit_plan_ref": "edit-plan:demo",
    "current_content_hash": "content-hash:demo"
  },
  "idempotency_key": "patch-plan-demo"
}
```

Unavailable diagnostic:

```json
{
  "pack_id": "pack.developer.code.v1",
  "required": false,
  "reason_code": "optional_pack_unresolved",
  "message": "developer_code_provider_not_installed"
}
```

## App-Facing Example Matrix

Generic examples cover workspace inspection, document parsing, symbol lookup,
diagnostics, code-action discovery, edit planning, patch generation, patch
validation, patch-apply request planning, diff inspection, impact estimation,
test suggestion, and scan-result import. All examples use synthetic workspace,
document, edit-plan, scan, and content-hash refs.

Diagnostic examples cover unavailable provider, missing workspace permission,
unsupported language, stale index, patch conflict, and approval-required
outcomes. Diagnostics must use provider-neutral reason codes and must not
include provider names, credentials, private source, repository-specific
conventions, raw patches, raw diffs, prompts, or application workflows.

## Provider Conformance

Provider authors must prove descriptor completeness, language and parser
support, index freshness, diagnostics and code-action compatibility, patch
safety, content-hash preconditions, rollback refs, scan import redaction,
resource bounds, policy hooks, sanitized trace/audit events, unavailable
behavior, snapshot/replay metadata, and no raw source, patch, diff, prompt, or
provider payload leakage.

## Trace And Audit

Trace and audit events may include pack id, service id, command name, trace id,
descriptor hashes, provider class, bounded counters, status, and trace-safe
error codes. They must not include raw source, raw patches, raw diffs, scanner
payloads, prompts, credentials, or provider payloads.

## Provider Replacement

Provider classes are descriptor labels such as `language-intelligence`,
`patch-planner`, `scan-adapter`, `mock`, and `unavailable`. Parser engines,
language servers, scanners, model clients, and patch appliers stay behind
service adapters selected by approved composition roots.
