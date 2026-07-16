## ADDED Requirements

### Requirement: Macaca SHALL provide the Developer Browser Automation Pack as a serviceized capability

Macaca SHALL provide `pack.developer.browser.automation.v1` as a provider-neutral industrial pack for isolated browser contexts, pages, frames, navigation, wait conditions, DOM/query inspection, locator resolution, user actions, script evaluation, screenshots, accessibility snapshots, downloads/uploads, console/dialog/network/trace event inspection, storage-state handles, cleanup, and replay diagnostics. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.developer.browser.automation.v1` as required and browser automation service provider is registered, healthy, entitled, origin-scoped, host-capable, and policy-admissible
- **THEN** admission SHALL expose `pack.developer.browser.automation.v1` in the effective capability set with command schemas, permission scopes, origin scope metadata, policy template hash, provider capability hash, health, and replay metadata
- **AND** SDK discovery SHALL mark callable `browser.*` commands as available without exposing provider secrets, cookies, credentials, raw screenshots, raw DOM dumps, raw downloads/uploads, network payloads, raw provider payloads, or application-specific workflow names

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.developer.browser.automation.v1` as required but provider, browser support, origin permission, entitlement, resource, approval, network, host support, or policy admission is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, create browser contexts, navigate, click, type, evaluate scripts, screenshot, upload, download, read storage, contact a network, or fake success

#### Scenario: Optional declaration degrades explicitly
- **WHEN** an application declares `pack.developer.browser.automation.v1` as optional and the pack or a sub-capability is unavailable
- **THEN** admission SHALL produce a degraded effective capability memento naming unavailable commands and bounded reason codes
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands while preserving discoverability and diagnostics

### Requirement: Browser automation commands SHALL use typed canonical service calls

Every `pack.developer.browser.automation.v1` operation SHALL be represented as a typed command/result DTO and SHALL traverse the canonical service runtime path with trace, policy, resource, entitlement, approval, health, snapshot, and structured error behavior. SDK, WASM ABI, shell, and application-framework helpers SHALL only build canonical service commands and SHALL NOT construct concrete browser providers or call browser APIs directly.

#### Scenario: Inspect command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `browser.inspect_provider`, `browser.inspect_dom`, `browser.inspect_events`, or `browser.capture_accessibility` is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and browser automation service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers and bounded cursor/artifact metadata

#### Scenario: Context creation is planned before request
- **WHEN** an application wants to create an isolated browser context/session
- **THEN** Macaca SHALL require `browser.plan_context` with context profile validation, storage policy, origin allowlist, network policy, artifact policy, resource reservation, credential reference, idempotency key, approval state where required, and provider capability validation
- **AND** `browser.plan_context` SHALL be replay-addressable and SHALL NOT create a browser context

#### Scenario: Side-effect command is denied before provider invocation
- **WHEN** policy, permission, entitlement, approval, resource, quota, origin, stale-handle, locator, actionability, script sandbox, artifact, storage, provider capability, or timeout checks reject a `browser.*` command
- **THEN** Macaca SHALL return a typed denied, validation, conflict, stale-handle, not-found, ambiguous-locator, navigation-failed, actionability-failed, script-denied, artifact-denied, storage-denied, approval-required, quota, timeout, unavailable, or unsupported result before invoking the concrete provider
- **AND** the audit trail SHALL include only bounded reason codes and sanitized handles

### Requirement: Browser automation DTOs SHALL model provider-neutral browser concepts

`pack.developer.browser.automation.v1` SHALL define provider-neutral DTOs for automation scope, provider capability, context profile, page, frame, locator, navigation plan, action plan, evaluation plan, wait condition, artifact handle, network event, console event, dialog event, trace event, storage handle, session snapshot, and diagnostics. Provider-specific fields SHALL be exposed only as bounded `adapter_metadata` guarded by capability hashes and SHALL NOT drive OS-layer routing branches.

#### Scenario: Provider capability is inspected
- **WHEN** `browser.inspect_provider` is invoked for a provider or origin scope
- **THEN** Macaca SHALL return provider-neutral `BrowserProviderCapability` metadata for browser engines, context support, page support, frame support, locator support, action support, evaluation support, screenshot support, accessibility support, download/upload support, network/log support, tracing support, storage support, auth modes, rate limits, lifecycle, health, and compatibility
- **AND** it SHALL include stable descriptor, provider capability, policy template, and compatibility hashes for validation and replay

#### Scenario: Page state is inspected
- **WHEN** a page-related command returns page state
- **THEN** the result SHALL use `BrowserPage`, context handle, URL handle, title handle, lifecycle state, frame summary, viewport, load state, freshness metadata, and redaction class
- **AND** it SHALL NOT expose raw cookies, credentials, storage values, raw DOM, raw screenshots, private network payloads, raw provider payloads, or website-specific private data

#### Scenario: Provider-specific capability exists
- **WHEN** an active provider supports a browser concept not present in the canonical DTO model
- **THEN** the provider MAY expose bounded `adapter_metadata` and compatibility diagnostics through `BrowserProviderCapability`
- **AND** the OS, SDK, shell, and generic application framework SHALL NOT branch on provider names, browser engines, website domains, selector strings, test workflows, or business actions

### Requirement: Context creation, navigation, actions, and evaluation SHALL be planned, requested, policy-safe, and auditable

All browser side effects SHALL use typed plans and/or requests, origin policy validation, page/frame/locator freshness validation, provider capability validation, resource reservations, idempotency where applicable, approval gates where required, and sanitized audit.

#### Scenario: Browser context is opened
- **WHEN** `browser.plan_context` validates isolation mode, storage policy, origin allowlist, network policy, artifact policy, resources, quota, credentials, and approvals
- **THEN** `browser.open_context_request` MAY use the validated plan handle and idempotency key to request context creation
- **AND** Macaca SHALL record sanitized plan, request, context profile hash, provider capability hash, policy decision, audit reason, context handle, and replay pointer

#### Scenario: Navigation is requested
- **WHEN** `browser.navigate` is invoked with a page handle and URL handle
- **THEN** Macaca SHALL validate origin policy, network policy, method/payload class, page freshness, wait condition, timeout, resource budget, approval state, and provider capability before navigation
- **AND** cross-origin, authenticated, financial, identity, external-recipient, or irreversible navigation flows SHALL be approval-gated when policy requires approval

#### Scenario: User action is requested
- **WHEN** `browser.plan_action` validates target locator, frame/page scope, visibility, stability, actionability, input payload, side-effect class, freshness, approvals, and provider capability
- **THEN** `browser.action_request` MAY request the validated click/type/fill/select/keyboard/mouse/touch action through the service provider
- **AND** Macaca SHALL return typed ambiguous-locator, actionability-failed, stale-handle, denied, or approval-required diagnostics before provider invocation when validation fails

#### Scenario: Script evaluation is requested
- **WHEN** `browser.plan_evaluate` validates sandbox profile, script handle, argument handles, timeout, origin policy, output policy, approval state, and provider capability
- **THEN** `browser.evaluate_request` MAY request bounded script evaluation
- **AND** raw script output, cookies, storage values, DOM dumps, credentials, local file contents, and provider payloads SHALL NOT enter observability

### Requirement: Browser artifacts, storage, events, and local files SHALL be bounded and policy-controlled

`pack.developer.browser.automation.v1` SHALL treat screenshots, accessibility snapshots, downloads, uploads, console events, dialog events, network events, traces, storage state, cookies, and local-file handles as policy-controlled resources with explicit permissions, quotas, redaction, retention, and provider capability checks.

#### Scenario: Screenshot or accessibility artifact is captured
- **WHEN** `browser.capture_screenshot` or `browser.capture_accessibility` is invoked
- **THEN** Macaca SHALL validate page/frame scope, viewport/full-page policy, sensitivity, redaction, size class, retention, provider capability, resource budget, and approval requirements
- **AND** it SHALL return a bounded `BrowserArtifactHandle` rather than raw screenshot pixels or raw accessibility payloads in traces, audits, snapshots, examples, or diagnostics

#### Scenario: Download or upload is managed
- **WHEN** `browser.manage_download` or `browser.manage_upload` is invoked
- **THEN** Macaca SHALL validate download/upload permission, local-file handle policy, origin policy, file size class, content type, retention, redaction, approval requirements, and provider capability
- **AND** it SHALL return structured denied or unsupported diagnostics before file transfer when policy or provider support is absent

#### Scenario: Browser events are inspected
- **WHEN** `browser.inspect_events` is invoked
- **THEN** Macaca SHALL return bounded console, dialog, network, request/response, and trace events with sanitized handles, cursor metadata, resource class, timing class, redaction class, and replay pointer
- **AND** it SHALL enforce event count, page size, network payload redaction, retention, timeout, and replay bounds

#### Scenario: Storage state is managed
- **WHEN** `browser.manage_storage_state` exports, imports, or deletes storage handles
- **THEN** Macaca SHALL validate storage permission, origin scope, sensitivity class, retention, credential policy, approval state, and provider capability
- **AND** cookies, tokens, local storage values, and session storage values SHALL be represented as sensitive handles rather than raw observability payloads

### Requirement: Browser Automation Pack SHALL enforce permissions, scopes, resources, entitlements, approvals, and redaction

`pack.developer.browser.automation.v1` SHALL enforce explicit permission scopes for provider inspection, context open/close, page open/close, navigation, waits, DOM inspection, locator resolution, action performance, script evaluation, screenshots, accessibility inspection, download management, upload management, event inspection, and storage management. Every command SHALL carry application id, tenant id, session id, task id, trace id, provider scope, context/page/frame handle where applicable, origin handle, and actor handle when available.

#### Scenario: Permission is missing
- **WHEN** an application invokes a `browser.*` command without the required permission scope
- **THEN** Macaca SHALL return a typed denied result before provider invocation
- **AND** the denied result SHALL identify the missing permission scope using sanitized identifiers

#### Scenario: Resource budget is exceeded
- **WHEN** context creation, navigation, actions, evaluation, screenshots, downloads, uploads, event inspection, storage management, or cleanup exceeds context count, page count, frame count, timeout, artifact size, download/upload size, network bytes, event count, memory, CPU, provider quota, or snapshot retention budgets
- **THEN** Macaca SHALL return typed quota, timeout, cancellation, artifact-denied, storage-denied, or resource-denied diagnostics
- **AND** it SHALL preserve replayable audit evidence without raw artifacts or provider payloads

#### Scenario: Sensitive operation requires approval
- **WHEN** policy marks authenticated pages, credential/storage access, cross-origin navigation, form submission, payment/financial or identity actions, downloads/uploads, local-file access, script evaluation, destructive actions, external side effects, screenshot/export of sensitive pages, or storage export/import as approval-required
- **THEN** Macaca SHALL return an approval-required result until a valid approval token is supplied
- **AND** no context creation, navigation, action, evaluation, screenshot, upload, download, storage access, network access, or local-file access SHALL happen before approval

### Requirement: Browser Automation Pack SHALL expose industrial metadata and developer documentation

`pack.developer.browser.automation.v1` SHALL expose descriptor metadata for command schemas, permission scopes, policy templates, resource budgets, approval rules, redaction profiles, provider capability hashes, SDK examples, lifecycle state, compatibility, health probes, snapshots, unavailable diagnostics, and documentation links. The implementation SHALL include detailed developer documentation at `docs/developer-packs/developer/browser-automation.md`.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.developer.browser.automation.v1`
- **THEN** it SHALL return command namespace `browser.*`, command schemas, permissions, provider/browser support, context/page/frame support, locator/action support, evaluation support, screenshot/accessibility support, download/upload support, event/network/tracing support, storage support, examples, lifecycle, availability, health, diagnostics, compatibility metadata, redaction profiles, and documentation link
- **AND** examples SHALL use synthetic pages, URLs, locators, artifacts, events, and storage handles rather than provider names, credentials, cookies, private DOM, screenshots, downloads, network payloads, or website-specific workflows

#### Scenario: Developer documentation is complete
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/developer/browser-automation.md` SHALL document manifest declarations, required versus optional behavior, permissions, provider scopes, browser contexts, pages, frames, locators, navigation, waits, DOM inspection, actions, evaluation, screenshots, accessibility, downloads, uploads, events, storage state, traces, cleanup, command DTOs, result DTOs, idempotency, artifact/pagination/streaming behavior, timeout/cancellation, redaction, approvals, unavailable diagnostics, provider replacement, trace/audit interpretation, conformance tests, and supplier/API mapping
- **AND** the guide SHALL be linked from SDK discovery metadata and the industrial pack catalog index

### Requirement: Browser Automation Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.developer.browser.automation.v1` SHALL emit sanitized trace and audit events for declaration, admission, provider inspection, context planning, context open requests, page opening, navigation, waits, DOM inspection, locator resolution, action planning, action requests, evaluation planning, evaluation requests, screenshot capture, accessibility capture, download/upload management, event inspection, storage management, page closing, context closing, policy decisions, service-call lifecycle, failures, unavailable states, and snapshots.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.developer.browser.automation.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, context/page state summaries, origin policy hash, artifact policy hash, command availability, provider health, resource counters, event cursors, artifact summaries, storage handle summaries, and sanitized replay pointers
- **AND** it SHALL exclude raw cookies, credentials, storage values, local file contents, raw screenshots, raw DOM dumps, raw downloads/uploads, network payloads, raw provider payloads, prompts, manifests, package bytes, private keys, signatures, and unbounded logs

#### Scenario: Replay reconstructs command evidence
- **WHEN** replay inspects a past `browser.*` command
- **THEN** Macaca SHALL reconstruct descriptor version, command DTO hash, policy decision, resource decision, approval state, provider capability hash, context/page/frame handles, plan handle where applicable, artifact/event cursor where applicable, result classification, and sanitized provider class metadata
- **AND** replay SHALL NOT require raw provider payloads, cookies, credentials, raw screenshots, DOM dumps, downloads/uploads, network payloads, local file contents, or application-specific workflow code

### Requirement: Browser automation implementation SHALL preserve Macaca boundaries

The `pack.developer.browser.automation.v1` implementation SHALL remain owned by browser automation service providers and service-runtime contracts. The microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific, supplier-specific, browser-specific, website-specific, selector-specific, or workflow-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, canonical execution-path, and serviceization gates scan the implementation
- **THEN** they SHALL find no concrete Playwright, Puppeteer, Selenium, CDP, WebDriver, browser engine, remote-grid, profile-store, credential-manager, filesystem-provider, network-provider, or website-specific adapter imports in the microkernel, SDK helpers, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.developer.browser.automation.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hashes, and bounded diagnostics rather than provider-specific business branches
