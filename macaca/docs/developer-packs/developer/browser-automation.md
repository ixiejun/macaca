# Developer Browser Automation Pack

`pack.developer.browser.automation.v1` provides provider-neutral browser
context planning, context opening, page opening, navigation, waits, DOM
inspection, locator resolution, action planning, action requests, evaluation
planning, evaluation requests, screenshot and accessibility capture, download
and upload management, event inspection, storage-state management, page close,
context close, and provider capability discovery.

The pack exposes browser automation as an audited service capability.
Applications receive page, frame, locator, action, artifact, event, storage,
and snapshot references instead of direct browser runtime objects.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.developer.browser.automation.v1"]
```

Unavailable optional declarations report
`developer_browser_automation_provider_not_installed`. Required declarations
block readiness until a descriptor-compatible browser automation provider is
installed.

## Permission Scopes

- `browser.provider.inspect`, `browser.context.open`,
  `browser.context.close`, `browser.page.open`, and `browser.page.close`.
- `browser.navigate`, `browser.wait`, `browser.dom.inspect`,
  `browser.locator.resolve`, `browser.action.perform`, and
  `browser.evaluate`.
- `browser.screenshot`, `browser.accessibility.inspect`,
  `browser.download.manage`, `browser.upload.manage`,
  `browser.events.inspect`, and `browser.storage.manage`.

## Commands

- `browser.inspect_provider`, `browser.plan_context`,
  `browser.open_context_request`, `browser.open_page`,
  `browser.navigate`, and `browser.wait_for`.
- `browser.inspect_dom`, `browser.resolve_locator`, `browser.plan_action`,
  `browser.action_request`, `browser.plan_evaluate`, and
  `browser.evaluate_request`.
- `browser.capture_screenshot`, `browser.capture_accessibility`,
  `browser.manage_download`, `browser.manage_upload`,
  `browser.inspect_events`, `browser.manage_storage_state`,
  `browser.close_page`, and `browser.close_context`.

## DTOs And Results

Core DTOs include `BrowserAutomationScope`, `BrowserProviderCapability`,
`BrowserContextProfile`, `BrowserPage`, `BrowserFrame`, `BrowserLocator`,
`BrowserNavigationPlan`, `BrowserActionPlan`, `BrowserEvaluationPlan`,
`BrowserWaitCondition`, `BrowserArtifactHandle`, `BrowserNetworkEvent`,
`BrowserConsoleEvent`, `BrowserDialogEvent`, `BrowserTraceEvent`,
`BrowserStorageHandle`, and `BrowserSessionSnapshot`. Result statuses cover
success, streaming, paging, partial results, denied, unavailable, unsupported,
conflict, stale handles, not found, ambiguous locators, navigation failure,
actionability failure, script denial, artifact denial, storage denial, quota,
timeout, cancellation, approval required, and provider failure.

## Command DTO Details

Every command wrapper carries a `DeveloperCommandEnvelope`:

- `subject_ref`: browser scope, context profile, context, page, frame, locator,
  navigation plan, action plan, evaluation plan, wait condition, artifact,
  event, storage handle, or snapshot subject.
- `parameters`: reference-only arguments such as `context_ref`, `page_ref`,
  `frame_ref`, `locator_ref`, `action_plan_ref`, `evaluation_plan_ref`,
  `artifact_ref`, `storage_ref`, and `approval_ref`.
- `cursor` and `page_size`: bounded pagination for events, accessibility
  snapshots, network records, console records, and trace records.
- `idempotency_key`: stable key for context open, navigation, action,
  evaluation, artifact, storage, and cleanup requests.

Result envelopes return `status`, optional `data`, optional paged data, and a
trace-safe error. Context, action, and evaluation side effects are split into
plan and request phases; artifacts are returned as handles with retention
metadata rather than raw bytes.

## Supplier/API Mapping

- Playwright browser, context, page, frame, locator, actionability, wait,
  screenshot, download/upload, storage-state, trace, and accessibility concepts
  map to provider-neutral browser DTOs.
- Chrome DevTools Protocol target, session, runtime evaluation, DOM, network,
  console, screenshot, storage, and tracing concepts map to refs and bounded
  event records.
- W3C WebDriver BiDi browsing context, navigation, script, log, network, and
  input concepts map to context, page, action, evaluation, and event refs.
- Selenium WebDriver session, element, navigation, action, script, and window
  concepts map to the same abstraction.
- Website-specific workflows, cookies, raw DOM, credentials, screenshots,
  downloads, and network payloads remain outside OS semantics.

## Examples

Plan a browser context:

```json
{
  "subject_ref": "browser-scope:demo",
  "parameters": { "context_profile_ref": "context-profile:demo" },
  "idempotency_key": "browser-demo-context-plan"
}
```

Plan an action against a locator:

```json
{
  "subject_ref": "page:demo",
  "parameters": {
    "locator_ref": "locator:primary-action",
    "action_kind": "click"
  },
  "idempotency_key": "browser-demo-action-plan"
}
```

Unavailable diagnostic:

```json
{
  "pack_id": "pack.developer.browser.automation.v1",
  "required": false,
  "reason_code": "optional_pack_unresolved",
  "message": "developer_browser_automation_provider_not_installed"
}
```

## App-Facing Example Matrix

Generic examples cover provider inspection, context planning, context request
planning, page opening, navigation, waits, DOM inspection by redacted refs,
locator resolution, action planning, action request planning, evaluation
planning, evaluation request planning, screenshot artifact handles,
download/upload artifact handles, event inspection, storage handles, and
cleanup. All examples use synthetic browser-scope, context, page, locator,
artifact, storage, and event refs.

Diagnostic examples cover unavailable provider, missing origin permission,
unsupported browser, ambiguous locator, actionability failed, script denied,
screenshot redacted, download denied, upload denied, storage approval,
navigation timeout, provider quota, network denied, and artifact-retention
outcomes. Diagnostics must use provider-neutral reason codes and must not
include provider names, credentials, cookies, private DOM, screenshots,
downloads, network payloads, scripts, storage values, or website-specific
conventions.

## Provider Conformance

Provider authors must prove descriptor completeness, context isolation, origin
policy, page/frame freshness, locator resolution, actionability validation,
evaluation sandboxing, artifact redaction, download/upload safety, storage
policy, event redaction, resource bounds, policy hooks, sanitized trace/audit
events, unavailable behavior, snapshot/replay metadata, and no cookie, DOM,
screenshot, download, upload, storage, credential, script, network, or provider
payload leakage.

## Trace And Audit

Trace and audit events may include context refs, page refs, locator refs,
action-plan refs, artifact handles, event refs, snapshot handles, bounded
counters, status, and trace-safe error codes. They must not include raw DOM,
screenshots, downloads, uploads, cookies, local storage values, credentials,
scripts, or provider payloads.

## Provider Replacement

Provider classes are descriptor labels such as `browser-runtime`,
`action-runtime`, `artifact-runtime`, `mock`, and `unavailable`. Concrete
browser engines, automation libraries, artifact stores, and storage-state
managers stay behind service adapters.
