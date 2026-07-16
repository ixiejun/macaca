## 1. Governance And Inventory

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, and current developer-pack platform change before implementation.
- [x] 1.2 Inventory existing service descriptors, SDK clients, optional packages, plugins, and workbench services that can back industrial pack entries.
- [x] 1.3 Record GitNexus CRITICAL/HIGH findings as memo only, per user instruction, before implementation commits.
- [x] 1.4 Confirm the required initial industrial pack list covers foundation, communication, knowledge, developer, office, media, finance, commerce, identity, location, device, AI, and workflow with the exact sub-pack set named in the proposal.

## 2. Catalog Contract

- [x] 2.1 Extend provider-neutral pack descriptor DTOs with explicit lifecycle, availability, command/result schema refs, examples, source attribution, and migration notes.
- [x] 2.2 Add executable Specification validators for callable versus unavailable entries, service-command mapping, lifecycle transitions, and sanitized diagnostic fields.
- [x] 2.3 Add deterministic catalog snapshot and effective capability mementos with stable hashes and replay references.
- [x] 2.4 Add unit tests for lifecycle states, unavailable planned packs, callable service mappings, and stable catalog hashes.

## 3. Child OpenSpec Proposal Track

- [x] 3.1 Maintain `child-proposals.md` as the authoritative index of required sub-pack proposals and their family grouping.
- [x] 3.2 Complete the `foundation` family task by creating, approving, implementing, and validating child proposals for `add-pack-foundation-filesystem`, `add-pack-foundation-key-value-state`, `add-pack-foundation-time`, `add-pack-foundation-random`, `add-pack-foundation-config`, `add-pack-foundation-secrets-reference`, and `add-pack-foundation-session-state`.
- [x] 3.3 Complete the `communication` family task by creating, approving, implementing, and validating child proposals for `add-pack-communication-email`, `add-pack-communication-messaging`, `add-pack-communication-notification`, `add-pack-communication-inbox`, and `add-pack-communication-calendar`.
- [x] 3.4 Complete the `knowledge` family task by creating, approving, implementing, and validating child proposals for `add-pack-knowledge-search`, `add-pack-knowledge-retrieval`, `add-pack-knowledge-document-parsing`, `add-pack-knowledge-citations`, `add-pack-knowledge-graph`, and `add-pack-knowledge-summarization`.
- [x] 3.5 Complete the `developer` family task by creating, approving, implementing, and validating child proposals for `add-pack-developer-code`, `add-pack-developer-repository`, `add-pack-developer-ci`, `add-pack-developer-issue-tracker`, `add-pack-developer-terminal`, `add-pack-developer-browser-automation`, and `add-pack-developer-design-tools`.
- [x] 3.6 Complete the `office` family task by creating, approving, implementing, and validating child proposals for `add-pack-office-document`, `add-pack-office-spreadsheet`, `add-pack-office-presentation`, `add-pack-office-pdf`, and `add-pack-office-forms`.
- [x] 3.7 Complete the `media` family task by creating, approving, implementing, and validating child proposals for `add-pack-media-image`, `add-pack-media-audio`, `add-pack-media-video`, `add-pack-media-transcription`, and `add-pack-media-rendering`.
- [x] 3.8 Complete the `finance` family task by creating, approving, implementing, and validating child proposals for `add-pack-finance-market-data`, `add-pack-finance-stock`, `add-pack-finance-crypto`, `add-pack-finance-accounting`, `add-pack-finance-portfolio`, and `add-pack-finance-invoice`.
- [x] 3.9 Complete the `commerce` family task by creating, approving, implementing, and validating child proposals for `add-pack-commerce-catalog`, `add-pack-commerce-cart`, `add-pack-commerce-order`, `add-pack-commerce-payment-intent`, `add-pack-commerce-receipt`, and `add-pack-commerce-entitlement`.
- [x] 3.10 Complete the `identity` family task by creating, approving, implementing, and validating child proposals for `add-pack-identity-account`, `add-pack-identity-profile`, `add-pack-identity-auth-handoff`, `add-pack-identity-organization`, and `add-pack-identity-tenant`.
- [x] 3.11 Complete the `location` family task by creating, approving, implementing, and validating child proposals for `add-pack-location-maps`, `add-pack-location-geocode`, `add-pack-location-route`, `add-pack-location-place-search`, and `add-pack-location-timezone`.
- [x] 3.12 Complete the `device` family task by creating, approving, implementing, and validating child proposals for `add-pack-device-sensors`, `add-pack-device-camera`, `add-pack-device-local-files`, `add-pack-device-notifications`, and `add-pack-device-foreground-background-host`.
- [x] 3.13 Complete the `ai` family task by creating, approving, implementing, and validating child proposals for `add-pack-ai-llm`, `add-pack-ai-embedding`, `add-pack-ai-rerank`, `add-pack-ai-vision`, `add-pack-ai-speech`, and `add-pack-ai-model-evaluation`.
- [x] 3.14 Complete the `workflow` family task by creating, approving, implementing, and validating child proposals for `add-pack-workflow-task`, `add-pack-workflow-schedule`, `add-pack-workflow-approval`, `add-pack-workflow-delegation`, `add-pack-workflow-review`, and `add-pack-workflow-recovery`.
- [x] 3.15 Reject umbrella completion unless each child proposal proves industrial-grade availability or explicitly records preview/unavailable scope with its own approved rationale.

## 4. SDK And Admission

- [x] 4.1 Extend SDK pack discovery to return availability, lifecycle, command schemas, examples, diagnostics, and migration notes without importing providers.
- [x] 4.2 Extend application admission to reject required unavailable industrial packs and degrade optional unavailable packs.
- [x] 4.3 Ensure SDK invocation helpers only build canonical traced `ServiceCallCommand` values for declared callable commands.
- [x] 4.4 Add tests proving shell and SDK discovery use facade-owned DTOs and never import optional package providers.

## 5. Runtime Composition

- [x] 5.1 Add generic catalog composition hooks for base descriptors, optional package descriptors, and plugin descriptors.
- [x] 5.2 Ensure runtime-host records sanitized provider snapshots and unavailable diagnostics for every industrial pack source.
- [x] 5.3 Add structured unavailable behavior for absent required and optional industrial packs.
- [x] 5.4 Add tests proving base runtime-host contains no industrial business-domain implementation.

## 6. Trace, Audit, And Gates

- [x] 6.1 Emit sanitized catalog composition, entry load, declaration validation, resolution, degraded, provider snapshot, policy decision, and service-call events.
- [x] 6.2 Add replay tests proving industrial pack resolution and invocation remain trace-addressable through the canonical service path.
- [x] 6.3 Add static gates for no application-specific, provider-specific, or business-domain routing branches in kernel, SDK, shells, and base runtime-host.
- [x] 6.4 Run OpenSpec validation, targeted cargo tests, dependency-boundary gates, file-size gates, and no-direct-provider-call gates before marking implementation complete.
