# Developer Design Tools Pack

`pack.developer.design.tools.v1` provides provider-neutral design workspace,
file, page, node, component, style, token, token sync, asset export, component
mapping, write-change planning, write-change request, review inspection,
artifact handle, and provider capability discovery.

The pack treats design tooling as a serviceized capability. Applications work
with design refs, token refs, export plans, change sets, mapping refs, review
events, and artifact handles rather than concrete design platform objects.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.developer.design.tools.v1"]
```

Unavailable optional declarations report
`developer_design_tools_provider_not_installed`. Required declarations block
readiness until a descriptor-compatible design tools provider is installed.

## Permission Scopes

- `design_tools.provider.inspect`, `design_tools.workspace.read`,
  `design_tools.file.read`, `design_tools.page.read`, and
  `design_tools.node.read`.
- `design_tools.component.read`, `design_tools.token.read`,
  `design_tools.token.write`, `design_tools.asset.export`,
  `design_tools.component.map`, `design_tools.design.write`,
  `design_tools.review.read`, and `design_tools.artifact.read`.

## Commands

- `design_tools.inspect_provider`, `design_tools.list_workspaces`,
  `design_tools.list_files`, `design_tools.open_file`,
  `design_tools.inspect_page`, and `design_tools.inspect_node`.
- `design_tools.inspect_components`, `design_tools.inspect_tokens`,
  `design_tools.plan_token_sync`, `design_tools.token_sync_request`,
  `design_tools.plan_asset_export`, and
  `design_tools.export_asset_request`.
- `design_tools.map_component`, `design_tools.plan_write_change`,
  `design_tools.write_change_request`, `design_tools.inspect_reviews`, and
  `design_tools.get_artifact_handle`.

## DTOs And Results

Core DTOs include `DesignToolScope`, `DesignToolProviderCapability`,
`DesignWorkspace`, `DesignFile`, `DesignPage`, `DesignNode`,
`DesignComponent`, `DesignStyle`, `DesignToken`, `DesignTokenSyncPlan`,
`DesignExportPlan`, `DesignArtifactHandle`, `DesignChangeSet`,
`DesignComponentMapping`, and `DesignReviewEvent`. Result statuses cover
success, paging, partial results, denied, unavailable, unsupported, conflict,
stale versions, schema mismatches, export denial, write denial, artifact
denial, quota, timeout, cancellation, approval required, and provider failure.

## Command DTO Details

Every command wrapper carries a `DeveloperCommandEnvelope`:

- `subject_ref`: design scope, workspace, file, page, node, component, style,
  token, token sync plan, export plan, artifact, change set, component mapping,
  or review event subject.
- `parameters`: reference-only arguments such as `workspace_ref`, `file_ref`,
  `page_ref`, `node_ref`, `component_ref`, `token_ref`, `export_plan_ref`,
  `change_set_ref`, `mapping_ref`, `version_hash`, and `approval_ref`.
- `cursor` and `page_size`: bounded pagination for workspaces, files, nodes,
  components, tokens, reviews, and artifacts.
- `idempotency_key`: stable key for token sync, asset export, write change, and
  artifact requests.

Result envelopes return `status`, optional `data`, optional paged data, and a
trace-safe error. Token sync, asset export, and write operations are split into
planning and request phases with version preconditions and artifact retention
metadata.

## Supplier/API Mapping

- Figma REST file, node, image export, component, style, comment, project,
  team, and auth concepts map to file, node, component, style, export, review,
  and artifact refs.
- Figma Plugin API document, node, component, instance, variable, style, export
  setting, and mutation concepts map to node, component, token, export, and
  change-set DTOs.
- Adobe Photoshop UXP document, layer, action, export, plugin manifest, and
  scripting concepts map to files, nodes, export plans, change sets, and
  artifact handles.
- Penpot and W3C DTCG token sets, aliases, themes, and JSON structures map to
  design tokens and token sync plans.
- Provider OAuth, desktop automation, raw design files, private comments,
  brand-specific workflows, and raw asset payloads remain provider-private.

## Examples

Inspect a design node:

```json
{
  "subject_ref": "design-file:demo",
  "parameters": {
    "page_ref": "page:main",
    "node_ref": "node:primary-button"
  },
  "idempotency_key": "design-demo-node-inspect"
}
```

Plan token sync:

```json
{
  "subject_ref": "design-workspace:demo",
  "parameters": {
    "token_ref": "token-set:core",
    "version_hash": "version:demo"
  },
  "idempotency_key": "design-demo-token-sync-plan"
}
```

Unavailable diagnostic:

```json
{
  "pack_id": "pack.developer.design.tools.v1",
  "required": false,
  "reason_code": "optional_pack_unresolved",
  "message": "developer_design_tools_provider_not_installed"
}
```

## App-Facing Example Matrix

Generic examples cover provider inspection, workspace and file discovery, file
opening, page/node inspection, component inspection, token inspection, token
sync planning, token sync request planning, asset export planning, asset export
request planning, component mapping, write planning, write request planning,
review inspection, and artifact handles. All examples use synthetic workspace,
file, page, node, component, token, mapping, review, and artifact refs.

Diagnostic examples cover unavailable provider, missing workspace permission,
stale version, token-schema mismatch, export denied, write approval, artifact
denied, provider quota, network denied, review redacted, and unsupported write
outcomes. Diagnostics must use provider-neutral reason codes and must not
include provider names, credentials, private comments, customer data, raw
assets, proprietary designs, raw design files, token secrets, or
workflow-specific conventions.

## Provider Conformance

Provider authors must prove descriptor completeness, workspace/file scope
validation, node bounds, component/style compatibility, token schema
validation, export validation, version conflict handling, write safety,
artifact redaction, review redaction, collaborator notification policy,
resource bounds, policy hooks, sanitized trace/audit events, unavailable
behavior, snapshot/replay metadata, and no raw credentials, token secrets,
private comments, customer data, raw design files, raw assets, or provider
payload leakage.

## Trace And Audit

Trace and audit events may include workspace refs, file refs, node refs, token
schema hashes, export-plan refs, change-set refs, mapping refs, artifact
handles, bounded counters, status, and trace-safe errors. They must not include
raw design payloads, token secret values, assets, credentials, comments, or raw
provider payloads.

## Provider Replacement

Provider classes are descriptor labels such as `design-read`, `token-sync`,
`write-export`, `mock`, and `unavailable`. Concrete design APIs, asset stores,
write executors, and token synchronization engines stay behind service
adapters.
