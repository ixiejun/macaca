## 1. Supplier API Research And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, OpenSpec rules, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Study Figma REST API for files, nodes, images, comments, components, component sets, styles, projects, teams, authentication, and rate limits.
- [x] 1.3 Study Figma Plugin API for document nodes, components, instances, variables, styles, export settings, and in-file mutations.
- [x] 1.4 Study Adobe Photoshop UXP APIs for documents, layers, actions, output/export, plugin manifests, and scripting access.
- [x] 1.5 Study Penpot design tokens and W3C DTCG token structures for token sets, themes, aliases, and JSON import/export.
- [x] 1.6 Produce a supplier capability comparison memo mapping Figma REST, Figma Plugin API, Adobe UXP, and Penpot token concepts into Macaca provider-neutral design tools DTOs and commands.
- [x] 1.7 Define explicit non-goals for concrete Figma, Adobe, Penpot, Sketch, OAuth, plugin-runtime, desktop automation, provider clients, design-to-code workflows, brand workflows, raw provider pass-through, and provider-specific routing.
- [x] 1.8 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.developer.design.tools.v1` descriptor metadata: pack id, family, lifecycle, stability, workspace/file support, node support, component support, style support, token support, export support, write support, comment/review support, auth modes, command schemas, permission scopes, policy templates, resource budgets, approval requirements, data-governance class, SDK metadata, documentation link, compatibility, and diagnostics.
- [x] 2.2 Define provider-neutral DTOs for `DesignToolScope`, `DesignToolProviderCapability`, `DesignWorkspace`, `DesignFile`, `DesignPage`, `DesignNode`, `DesignComponent`, `DesignStyle`, `DesignToken`, `DesignTokenSyncPlan`, `DesignExportPlan`, `DesignArtifactHandle`, `DesignChangeSet`, `DesignComponentMapping`, and `DesignReviewEvent`.
- [x] 2.3 Define typed command/result DTOs for `design_tools.inspect_provider`, `design_tools.list_workspaces`, `design_tools.list_files`, `design_tools.open_file`, `design_tools.inspect_page`, `design_tools.inspect_node`, `design_tools.inspect_components`, `design_tools.inspect_tokens`, `design_tools.plan_token_sync`, `design_tools.token_sync_request`, `design_tools.plan_asset_export`, `design_tools.export_asset_request`, `design_tools.map_component`, `design_tools.plan_write_change`, `design_tools.write_change_request`, `design_tools.inspect_reviews`, and `design_tools.get_artifact_handle`.
- [x] 2.4 Define typed success, paged, partial, denied, unavailable, unsupported, conflict, stale-version, schema-mismatch, export-denied, write-denied, artifact-denied, quota, timeout, cancellation, approval-required, and failure DTOs.
- [x] 2.5 Define stable descriptor hashing, provider capability hashing, workspace/file hashing, page/node hashing, component mapping hashing, token schema hashing, token sync plan hashing, export plan hashing, artifact handle hashing, change set hashing, review cursor hashing, and redaction metadata.
- [x] 2.6 Add descriptor and DTO compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, schema evolution, workspaces, files, pages, nodes, components, styles, tokens, token sync plans, export plans, change sets, mappings, review events, artifacts, redaction profiles, and serde compatibility.

## 3. Admission, Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement manifest declaration validation for required and optional `pack.developer.design.tools.v1` declarations.
- [x] 3.2 Implement permission validation for `design_tools.provider.inspect`, `design_tools.workspace.read`, `design_tools.file.read`, `design_tools.page.read`, `design_tools.node.read`, `design_tools.component.read`, `design_tools.token.read`, `design_tools.token.write`, `design_tools.asset.export`, `design_tools.component.map`, `design_tools.design.write`, `design_tools.review.read`, and `design_tools.artifact.read`.
- [ ] 3.3 Implement provider/workspace/file/page/node/component/token/artifact/review scope checks for declared workspaces, private files, unpublished libraries, customer data, comments, asset exports, denied scopes, and stale handles.
- [ ] 3.4 Implement policy checks for version preconditions, token schema validation, DTCG compatibility, component mapping compatibility, export format policy, node depth/output bounds, artifact retention, comment/review redaction, collaborator/notification policy, write change validation, and output redaction.
- [ ] 3.5 Implement resource reservation for file count, node depth, node count, component count, token count, export size, artifact size, comment/review count, payload size, provider quota, network transfer, timeout, memory, storage, and retained snapshots.
- [ ] 3.6 Implement entitlement checks and structured unavailable/denied diagnostics for missing provider, disabled pack, missing credential reference, missing workspace permission, unsupported node read, unsupported token sync, unsupported export format, unsupported write, disabled network, missing entitlement, provider quota, and host resource denial.
- [ ] 3.7 Implement approval behavior for private/unpublished files, brand libraries, customer data, private comments, token writes, component overwrites, collaborator-visible changes, destructive mutations, asset export, and operations that notify collaborators or external systems.
- [ ] 3.8 Add tests proving denied, validation, quota, unavailable, conflict, stale-version, schema-mismatch, export-denied, write-denied, artifact-denied, unsupported, timeout, cancellation, and approval-required paths do not call concrete providers, mutate files, sync tokens, export assets, overwrite components, notify collaborators, or expose raw design data.

## 4. Service Provider And Runtime Integration

- [ ] 4.1 Implement or bind the design-tool service provider behind the service runtime; do not construct design-tool providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add a deterministic unavailable provider that returns typed unavailable/unsupported diagnostics and complete discovery metadata.
- [ ] 4.3 Add mock provider support for provider inspection, workspace/file listing, file opening, page/node/component/token inspection, token sync planning/request, asset export planning/request, component mapping, write planning/request, review inspection, artifact handles, health, and provider capability inspection.
- [ ] 4.4 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, bounded paging, node depth limits, review cursors, stale-version diagnostics, schema-mismatch diagnostics, artifact retention, and rate-limit diagnostics.
- [ ] 4.5 Add Strategy implementations for provider adapters, node readers, token schema validators, export planners, change validators, component mapping validators, artifact providers, review readers, redaction, and unavailable behavior.
- [ ] 4.6 Add side-effect safety support for idempotency keys, provider state validation, file version preconditions, token schema compatibility checks, export preconditions, approval state, collaborator notification policy, artifact retention, and non-mutating plan commands.
- [ ] 4.7 Add provider capability reporting for available, degraded, preview, unavailable, unsupported, retired, workspace-limited, file-limited, node-limited, component-limited, token-limited, export-limited, write-limited, review-limited, network-limited, and quota-limited states.

## 5. SDK, WASM ABI, Application Framework, And Examples

- [x] 5.1 Extend SDK discovery for `pack.developer.design.tools.v1` with command schemas, workspace/file support, node support, component support, token support, export support, write support, review support, examples, availability, diagnostics, documentation link, provider class, capability hash, compatibility, and redaction profile.
- [x] 5.2 Extend application admission so required declarations block readiness when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders for all `design_tools.*` commands; helpers must only build canonical traced service calls and must never construct design-tool clients, access credentials, call provider APIs, mutate files, sync tokens, export raw assets, read private comments, or bypass policy.
- [ ] 5.4 Extend WASM/app ABI descriptors so applications can discover design tools commands, declare permissions, receive unavailable diagnostics, and submit typed service calls through the canonical execution path.
- [x] 5.5 Add generic app-facing examples for provider inspection, workspace/file discovery, file opening, page/node inspection, component inspection, token inspection, token sync planning/request, asset export planning/request, component mapping, write planning/request, review inspection, and artifact handles.
- [x] 5.6 Add unavailable-provider, missing-workspace-permission, stale-version, token-schema-mismatch, export-denied, write-approval, artifact-denied, provider-quota, network-denied, review-redacted, and unsupported-write examples that demonstrate diagnostics without provider names, credentials, private comments, customer data, raw assets, proprietary designs, or workflow-specific conventions.

## 6. Trace, Audit, Replay, Security, And Gates

- [ ] 6.1 Emit sanitized declaration, admission, provider-inspection, workspace-list, file-list, file-open, page-inspection, node-inspection, component-inspection, token-inspection, token-sync-plan, token-sync-request, asset-export-plan, asset-export-request, component-mapping, write-plan, write-request, review-inspection, artifact-handle, policy, entitlement, resource, approval, health, snapshot, unavailable, and failure events.
- [ ] 6.2 Ensure traces, audits, snapshots, SDK diagnostics, and examples exclude raw credentials, tokens, private comments, customer data, raw design files, raw image assets, raw provider payloads, prompts, manifests, package bytes, private keys, signatures, and unbounded node trees.
- [ ] 6.3 Add replay tests proving every `design_tools.*` command is trace-addressable through the canonical service path and that snapshots contain enough bounded metadata for recovery diagnostics.
- [ ] 6.4 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete Figma, Adobe, Penpot, Sketch, OAuth, plugin-runtime, desktop automation, credential-manager, asset-provider, or provider adapters.
- [ ] 6.5 Add no-direct-provider-call gates proving SDK helpers, WASM ABI handlers, app admission, web, CLI, and frontend paths route through descriptor-owned service commands.
- [ ] 6.6 Add boundary tests proving optional provider absence returns structured unavailable diagnostics and never crashes, hangs, silently falls back, mutates files, syncs tokens, exports assets, overwrites components, notifies collaborators, retrieves raw design data, contacts providers, or fakes success.
- [ ] 6.7 Run `openspec validate add-pack-developer-design-tools --strict`, targeted cargo tests, boundary gates, file-size gates, and audit replay checks before marking implementation complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/developer/design-tools.md` with purpose, capability model, manifest declaration, required versus optional behavior, permissions, provider scopes, workspaces, files, pages/canvases, nodes, components, component sets, instances, styles, variables, tokens, libraries, exports, mappings, change sets, comments/reviews, artifacts, unavailable diagnostics, provider replacement, and operational limits.
- [x] 7.2 Document all command DTOs and result DTOs with field-level explanations, idempotency semantics, redaction behavior, pagination behavior, timeout/cancellation behavior, plan/request behavior, approval behavior, artifact retention behavior, version preconditions, token schema compatibility, and structured error codes.
- [x] 7.3 Document supplier/API mapping: Figma REST API, Figma Plugin API, Adobe Photoshop UXP, and Penpot/W3C DTCG token concepts mapped to Macaca abstractions, including what is intentionally not exposed as OS semantics.
- [x] 7.4 Add generic examples for provider inspection, file discovery, node inspection, component inspection, token sync, asset export, component mapping, write changes, review inspection, artifact handles, and unavailable diagnostics using synthetic design data only.
- [x] 7.5 Add conformance checklist and test guidance for provider authors: descriptor completeness, file/node scope validation, token schema validation, export validation, version conflicts, write safety, artifact redaction, review redaction, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and redaction.
- [x] 7.6 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-developer-design-tools` complete.
