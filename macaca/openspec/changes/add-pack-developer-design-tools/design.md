# Developer Design Tools Pack Design

## Context

`pack.developer.design.tools.v1` exposes design-tool access as a Macaca OS
serviceized capability. It lets applications discover design workspaces, inspect
files/pages/canvases/nodes/components/styles/tokens, export assets, map design
components to code components, inspect comments/review metadata, plan token or
design changes, and request approved writes without embedding Figma, Adobe UXP,
Penpot, Sketch, provider credentials, provider node schemas, or
application-specific design workflows into generic OS layers.

Design systems are collaborative source-of-truth assets. Reads can leak product
plans or customer data, and writes can update shared components, tokens, exports,
or collaborator-visible comments. The pack therefore models changes as typed
plans and requests with version preconditions, token schema compatibility,
artifact redaction, approval, trace/audit evidence, replay, and provider
replacement.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Figma REST API | Files, nodes, images, comments, components, styles, projects, teams, authenticated/rate-limited API access | Design workspace, design file, node tree, export artifact, comment event, component/style metadata, provider capability |
| Figma Plugin API | Document nodes, components/instances, variables, styles, export settings, in-file mutations | Node, component, instance, variable collection, token, export plan, write plan, provider mutation capability |
| Adobe Photoshop UXP | Documents, layers, actions, output/export, plugin/scripting access | Design document, layer/node, action/change set, export artifact, provider capability |
| Penpot design tokens | W3C DTCG-aligned design token sets, themes, aliases, JSON import/export | Token schema, token set, theme, alias, compatibility validation, token sync plan |

The pack exposes provider-neutral contracts. Provider adapters translate to REST
APIs, plugin runtimes, desktop automation bridges, or remote design services. OS
layers must not branch on provider names, tool names, file names, component
names, token names, node types, or business design-system conventions.

## Goals

- Provide stable pack id `pack.developer.design.tools.v1` and command namespace
  `design_tools.*`.
- Support provider inspection, workspace/file discovery, page/canvas inspection,
  node inspection, component/library inspection, style/token inspection, token
  sync planning/request, asset export planning/request, component mapping,
  design change planning/request, comment/review inspection, artifact handles,
  health, snapshot, and replay.
- Preserve safety with workspace/file/node scope validation, version
  preconditions, token schema compatibility, export format policy, artifact
  retention, write approval, resource quotas, and sanitized audit.
- Keep concrete design-tool providers behind replaceable service providers.
- Require developer documentation at
  `docs/developer-packs/developer/design-tools.md`.

## Non-Goals

- Do not implement concrete Figma, Adobe, Penpot, Sketch, OAuth, plugin-runtime,
  desktop automation, or provider clients in this proposal.
- Do not define application-specific design-to-code, brand, marketing, design
  system, UI generation, review, release, or asset pipeline workflows.
- Do not execute repository, browser automation, media rendering, office, or
  notification semantics directly; those belong to separate packs/services and
  may be linked by handles.
- Do not expose raw credentials, access tokens, unpublished design files, private
  comments, customer data, raw image assets, raw provider payloads, prompts,
  manifests, package bytes, private keys, signatures, or unbounded node trees in
  observability.
- Do not silently mutate files, sync tokens, export assets, overwrite components,
  or notify collaborators without typed request, policy checks, version
  preconditions, and approval where required.

## Ownership And Boundaries

- Pack id: `pack.developer.design.tools.v1`.
- Family: `developer`.
- Backing service owner: design-tool service provider.
- SDK surface: `sdk.packs.developer.design.tools`.
- Command namespace: `design_tools.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, credential bridges,
  artifact stores, plugin/remote bridges, decorators, and sanitized diagnostics
  through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `design_tools.inspect_provider` | Inspect provider/workspace/library capability | Returns sanitized file, node, component, token, export, write, quota, and health metadata |
| `design_tools.list_workspaces` | List accessible teams/projects/workspaces/libraries | Requires workspace permission and bounded paging |
| `design_tools.list_files` | List design files in a workspace/project | Requires file permission, paging, and redaction |
| `design_tools.open_file` | Open or resolve a design file handle | Returns version/freshness, page/canvas summaries, and capability metadata |
| `design_tools.inspect_page` | Inspect page/canvas metadata | Requires output bounds and redaction |
| `design_tools.inspect_node` | Inspect node tree or selected node handles | Requires node scope, depth limits, field projection, and redaction |
| `design_tools.inspect_components` | Inspect components, component sets, instances, variants, and libraries | Requires library scope and compatibility hashes |
| `design_tools.inspect_tokens` | Inspect styles, variables, tokens, themes, aliases, and token schema | Requires token permission, schema validation, and redaction |
| `design_tools.plan_token_sync` | Plan token import/export/sync | Validates token schema, conflicts, aliases, themes, version preconditions, and approvals |
| `design_tools.token_sync_request` | Request validated token sync | Requires plan handle, idempotency key, write permission, and audit |
| `design_tools.plan_asset_export` | Plan asset export from nodes/components/pages | Validates format, scale, bounds, sensitivity, retention, and approvals |
| `design_tools.export_asset_request` | Request export artifact from a validated plan | Returns bounded artifact handle and audit metadata |
| `design_tools.map_component` | Map design component to code/component metadata | Requires mapping schema, version compatibility, and provider-neutral handles |
| `design_tools.plan_write_change` | Plan design node/component/style/file mutation | Validates change set, version preconditions, collaborators, notifications, and approvals |
| `design_tools.write_change_request` | Request validated design mutation | Requires plan handle, idempotency key, write permission, and audit |
| `design_tools.inspect_reviews` | Inspect comments, review notes, approvals, and change events where supported | Requires comment/review permission, redaction, and paging |
| `design_tools.get_artifact_handle` | Resolve export/snapshot artifact handle metadata | Requires artifact permission, retention, and redaction |

Every command must define typed command DTOs, typed success results, typed
paged/partial results, typed denied/unavailable/unsupported/conflict/
stale-version/schema-mismatch/export-denied/write-denied/quota/timeout/
cancellation/approval-required/failure results, redaction profile, idempotency
semantics for side effects, and replay metadata.

## DTO Model

Core DTOs:

- `DesignToolScope`: provider scope handle, workspace/project/library handle,
  credential reference, network policy, artifact policy, permission state, rate
  limit profile, and health.
- `DesignToolProviderCapability`: provider class, file support, node support,
  component support, style support, token support, export support, write support,
  comment/review support, auth modes, rate limits, lifecycle, and health.
- `DesignWorkspace`: workspace handle, name handle, organization/tenant handle,
  visibility, library support, file count class, and provider capability hash.
- `DesignFile`: file handle, workspace handle, name handle, version hash,
  freshness, page summaries, library references, permission state, and redaction
  class.
- `DesignPage`: page/canvas handle, file handle, name handle, child count class,
  viewport/bounds class, version hash, and redaction class.
- `DesignNode`: node handle, page/file handle, parent handle, node kind,
  name handle, bounds class, style references, component references, child count
  class, version hash, and redaction class.
- `DesignComponent`: component handle, component set handle, variant metadata,
  instance metadata, library handle, version hash, mapping hash, and redaction
  class.
- `DesignStyle`: style handle, style kind, token references, node references,
  value handle, version hash, and sensitivity class.
- `DesignToken`: token handle, token path, token type, value handle, alias
  target, theme/set handle, DTCG compatibility, version hash, and sensitivity
  class.
- `DesignTokenSyncPlan`: plan handle, source/target handles, token schema hash,
  conflict diagnostics, alias/theme validation, required approvals, idempotency
  key, and validation diagnostics.
- `DesignExportPlan`: plan handle, source node/page/component handles, format,
  scale/density, bounds, color/profile policy, retention, redaction, approvals,
  idempotency key, and validation diagnostics.
- `DesignArtifactHandle`: artifact handle, source handle, artifact kind, content
  type, size class, checksum handle, retention, redaction class, and replay
  pointer.
- `DesignChangeSet`: change set handle, file/node/component/style/token
  operations, version preconditions, collaborator/notification policy, approval
  state, idempotency key, and validation diagnostics.
- `DesignComponentMapping`: mapping handle, design component handle, code
  component handle, prop/token mapping, variant mapping, compatibility hash, and
  diagnostics.
- `DesignReviewEvent`: event handle, file/node handle, event kind, actor handle,
  timestamp, comment/review handle, changed fields, redaction class, and cursor.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `design_tools.provider.inspect`
- `design_tools.workspace.read`
- `design_tools.file.read`
- `design_tools.page.read`
- `design_tools.node.read`
- `design_tools.component.read`
- `design_tools.token.read`
- `design_tools.token.write`
- `design_tools.asset.export`
- `design_tools.component.map`
- `design_tools.design.write`
- `design_tools.review.read`
- `design_tools.artifact.read`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, workspace/file/page/node handle, and actor handle
  when available.
- Token sync, asset export, and design writes require plan/request separation,
  idempotency key, version preconditions, schema validation, artifact policy,
  notification/collaborator policy, credential reference, and audit reason.
- Private workspaces, unpublished files, brand libraries, customer data,
  comments/reviews, token writes, component overwrites, collaborator-visible
  changes, asset exports, and destructive mutations may require approval.
- Node trees, comments, artifacts, and token values require redaction and
  bounded output. Raw design files and raw image assets must not enter
  observability.
- Remote operations require network permission, provider quota, rate limits,
  timeout, cancellation, and structured unavailable behavior.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
workspace/file support, node support, component support, token support, export
support, write support, review support, permission scopes, policy templates,
resource limits, approval rules, provider capability hashes, health,
compatibility, diagnostics, examples, redaction profiles, and documentation
links.

The developer guide at `docs/developer-packs/developer/design-tools.md` must
cover:

- manifest declaration and optional/required behavior
- provider scopes, workspaces, files, pages/canvases, nodes, components,
  component sets, instances, styles, variables, tokens, libraries, exports,
  mappings, change sets, comments/reviews, artifacts, provider capabilities, and
  unavailable states
- token sync plan/request lifecycle, asset export plan/request lifecycle, write
  change plan/request lifecycle, version conflicts, schema mismatch, artifact
  redaction, notification policy, approvals, quotas, provider replacement,
  trace/audit interpretation, and conformance tests

Examples must use synthetic workspaces, files, nodes, components, tokens, and
artifacts. They must not include provider names, real credentials, private
comments, customer data, raw assets, proprietary designs, or workflow-specific
conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `design_tools_pack_declared`
- `design_tools_pack_admission_validated`
- `design_tools_provider_inspected`
- `design_tools_workspaces_listed`
- `design_tools_files_listed`
- `design_tools_file_opened`
- `design_tools_page_inspected`
- `design_tools_node_inspected`
- `design_tools_components_inspected`
- `design_tools_tokens_inspected`
- `design_tools_token_sync_planned`
- `design_tools_token_sync_requested`
- `design_tools_asset_export_planned`
- `design_tools_asset_export_requested`
- `design_tools_component_mapped`
- `design_tools_write_change_planned`
- `design_tools_write_change_requested`
- `design_tools_reviews_inspected`
- `design_tools_artifact_handle_resolved`
- `design_tools_pack_policy_decision`
- `design_tools_pack_service_call_requested`
- `design_tools_pack_service_call_succeeded`
- `design_tools_pack_service_call_failed`
- `design_tools_pack_unavailable`
- `design_tools_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, workspace/file
schema hashes, token schema hashes, command availability, provider health,
policy template hash, resource counters, bounded file/node/component/token
summaries, artifact summaries, review cursors, and sanitized replay pointers.
Snapshots must exclude raw credentials, tokens, private comments, customer data,
raw design files, raw image assets, raw provider payloads, prompts, manifests,
package bytes, private keys, signatures, and unbounded node trees.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, node readers, token schema validators, export
  planners, change validators, component mapping validators, redaction, artifact
  retention, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  network policy, credential redaction, artifact redaction, and mutation safety
  wrap service calls.
- **Specification**: admission validates provider scope, workspace/file support,
  command availability, permissions, version preconditions, token schema,
  provider state, quota, and compatibility.
- **Observer**: design changes, comments/reviews, provider health, trace, and
  audit events are subscribable.
- **Memento**: file version hashes, token schema hashes, export plans, change
  sets, artifact handles, review cursors, snapshots, and replay pointers
  preserve recovery state.
- **Abstract Factory**: concrete design-tool providers are created only by
  approved runtime-host composition roots.

## Risks And Mitigations

- Risk: pack becomes a Figma or Adobe wrapper. Mitigation: provider-neutral
  workspace/file/page/node/component/token/artifact DTOs and Strategy adapters.
- Risk: unpublished designs or raw assets leak. Mitigation: handles, redaction,
  bounded summaries, and strict observability exclusions.
- Risk: design writes corrupt shared source-of-truth assets. Mitigation:
  plan/request split, version preconditions, schema validation, approval, and
  audit.
- Risk: token semantics diverge across providers. Mitigation: explicit token
  schema DTO, DTCG compatibility metadata, conflict diagnostics, and conformance
  tests.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call design-tool APIs directly.
