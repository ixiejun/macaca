# Developer Browser Automation Pack Design

## Context

`pack.developer.browser.automation.v1` exposes browser automation as a Macaca OS
serviceized capability. It lets applications create isolated browser contexts,
open pages, navigate, inspect DOM state, resolve locators, perform user actions,
evaluate scripts, capture screenshots and accessibility snapshots, observe
network/console/dialog events, handle uploads/downloads, collect traces, and
cleanup sessions without embedding Playwright, Puppeteer, Selenium, CDP,
WebDriver BiDi, browser-engine names, website-specific selectors, or
application-specific workflows into generic OS layers.

Browser automation is high-risk because it can cross identity, payment, privacy,
network, filesystem, and device boundaries. The pack therefore models browser
work as typed plans and requests, isolates sessions by context, bounds artifacts,
uses provider-neutral handles, redacts sensitive data, and requires policy,
entitlement, resource, approval, trace, audit, replay, and provider replacement
for every operation.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Playwright | Browser, BrowserContext, Page, Locator, auto-waiting actions, tracing, screenshots, downloads, network events | Browser context, page, locator, action plan, wait condition, trace artifact, screenshot/download handles |
| Chrome DevTools Protocol | Page, Runtime, DOM, Network, Input, Target, Browser, tracing/debugging domains | Provider command adapter, browsing target, DOM snapshot, script evaluation, network event, input action, trace event |
| W3C WebDriver BiDi | Browsing contexts, script, input, network, log, storage, event subscription | Browser context/page/frame, script evaluation plan, input action, network/log event stream, storage handle |
| Selenium WebDriver | Browser automation via local/remote WebDriver, element interaction, navigation, window/session lifecycle | Provider-neutral session, page/window handle, element/locator handle, navigation, user action, remote provider capability |

The pack exposes provider-neutral contracts. Provider adapters translate to
Playwright, Puppeteer, Selenium, CDP, WebDriver BiDi, local browsers, or remote
grids. OS layers must not branch on provider names, browser engines, website
domains, selector strings, test workflows, or business actions.

## Goals

- Provide stable pack id `pack.developer.browser.automation.v1` and command
  namespace `browser.*`.
- Support provider inspection, context planning/creation, page creation,
  navigation, wait conditions, DOM/query inspection, locator resolution,
  user-action planning/request, script evaluation planning/request, screenshot
  handles, accessibility snapshot handles, download/upload handles, console/log
  inspection, network event inspection, dialog handling, tracing, storage state
  handles, session cleanup, health, snapshot, and replay.
- Preserve safety with origin allowlists, credential/storage policy, sandboxed
  evaluation, local-file and download policy, network policy, artifact redaction,
  action approval, quotas, and audit.
- Keep concrete browser providers behind replaceable service providers.
- Require developer documentation at
  `docs/developer-packs/developer/browser-automation.md`.

## Non-Goals

- Do not implement concrete Playwright, Puppeteer, Selenium, CDP, WebDriver,
  browser engine, remote grid, profile store, or extension providers in this
  proposal.
- Do not define application-specific login, checkout, scraping, testing,
  administration, crawler, captcha, payment, support, or website workflows.
- Do not execute repository, terminal, CI, credential, filesystem, notification,
  or payment semantics directly; those belong to separate packs/services and may
  be linked by handles.
- Do not expose raw cookies, credentials, storage state, local file contents,
  screenshots, DOM dumps, downloads, uploads, network payloads, provider
  payloads, prompts, manifests, package bytes, private keys, signatures, or
  unbounded browser logs in observability.
- Do not silently navigate, click, type, upload, download, evaluate scripts, or
  submit forms without typed request, policy checks, and approval where required.

## Ownership And Boundaries

- Pack id: `pack.developer.browser.automation.v1`.
- Family: `developer`.
- Backing service owner: browser automation service provider.
- SDK surface: `sdk.packs.developer.browser.automation`.
- Command namespace: `browser.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, browser process bridges,
  remote/grid bridges, artifact stores, decorators, and sanitized diagnostics
  through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `browser.inspect_provider` | Inspect provider/browser capability | Returns sanitized browser, context, page, action, artifact, network, storage, quota, and health metadata |
| `browser.plan_context` | Plan isolated context/session creation | Validates profile, storage, permissions, origin policy, artifact policy, resources, and approvals |
| `browser.open_context_request` | Request context/session creation from a validated plan | Requires idempotency key, provider state, resource reservation, and audit |
| `browser.open_page` | Open a page/tab within a context | Requires context permission, page quota, and provider capability |
| `browser.navigate` | Navigate a page to a URL | Requires origin policy, method/input validation, wait policy, timeout, and approval when needed |
| `browser.wait_for` | Wait for load, selector, network, event, or custom condition | Requires bounded timeout, polling/event strategy, and cancellation |
| `browser.inspect_dom` | Return bounded DOM/accessibility/query metadata | Requires redaction, selector/query policy, and output bounds |
| `browser.resolve_locator` | Resolve provider-neutral locator handles | Requires frame/page scope and ambiguity diagnostics |
| `browser.plan_action` | Plan click/type/fill/select/keyboard/mouse/touch actions | Validates target, visibility, stability, actionability, side-effect class, and approvals |
| `browser.action_request` | Request a validated user action | Requires plan handle, idempotency key, freshness, and audit |
| `browser.plan_evaluate` | Plan script evaluation | Validates sandbox, input handles, timeout, origin policy, and output redaction |
| `browser.evaluate_request` | Request validated script evaluation | Requires plan handle, capability, approval where needed, and bounded result |
| `browser.capture_screenshot` | Create screenshot artifact handle | Requires page/frame scope, viewport/full-page policy, redaction, and retention |
| `browser.capture_accessibility` | Create bounded accessibility snapshot handle | Requires redaction, output bounds, and retention |
| `browser.manage_download` | Inspect or retrieve download handle metadata | Requires download permission, file policy, retention, and approval where needed |
| `browser.manage_upload` | Submit an upload handle where policy allows | Requires local-file handle, origin policy, approval, and audit |
| `browser.inspect_events` | Inspect console, dialog, network, request/response, and trace events | Requires event filters, redaction, paging, and retention |
| `browser.manage_storage_state` | Export/import/delete cookie/storage handles where policy allows | Requires storage permission, sensitive data policy, and approval |
| `browser.close_page` | Close page/tab | Requires lifecycle state validation and audit |
| `browser.close_context` | Cleanup context/session resources and retained artifacts | Requires lifecycle state validation and snapshot/audit update |

Every command must define typed command DTOs, typed success results, typed
streaming/paged results, typed denied/unavailable/unsupported/conflict/
stale-handle/not-found/ambiguous-locator/navigation-failed/actionability-failed/
script-denied/quota/timeout/cancellation/approval-required/failure results,
redaction profile, idempotency semantics for side effects, and replay metadata.

## DTO Model

Core DTOs:

- `BrowserAutomationScope`: provider scope, browser profile handle, workspace
  handle, credential reference, origin policy, network policy, artifact policy,
  permission state, rate-limit profile, and health.
- `BrowserProviderCapability`: provider class, browser engines, context support,
  page support, frame support, locator support, action support, evaluation
  support, screenshot support, accessibility support, download/upload support,
  network/log support, tracing support, storage support, auth modes, rate
  limits, lifecycle, and health.
- `BrowserContextProfile`: profile handle, isolation mode, viewport/device
  emulation, locale/timezone, permissions, storage policy, proxy/network policy,
  download policy, artifact retention, and redaction class.
- `BrowserPage`: page handle, context handle, URL handle, title handle, lifecycle
  state, frame summary, viewport, load state, freshness, and redaction class.
- `BrowserFrame`: frame handle, page handle, parent frame handle, origin handle,
  URL handle, lifecycle state, and redaction class.
- `BrowserLocator`: locator handle, page/frame scope, selector strategy,
  accessible name/role handles, text handle, strictness, ambiguity state,
  provider mapping hash, and redaction class.
- `BrowserNavigationPlan`: plan handle, page handle, target URL handle, method
  class, payload handle, wait condition, timeout, origin policy decision,
  approval state, idempotency key, and validation diagnostics.
- `BrowserActionPlan`: plan handle, action kind, locator/page/frame handle,
  input payload handle, actionability checks, expected side-effect class,
  approval state, idempotency key, and validation diagnostics.
- `BrowserEvaluationPlan`: plan handle, page/frame handle, sandbox profile,
  script handle, argument handles, timeout, output policy, approval state, and
  validation diagnostics.
- `BrowserWaitCondition`: condition kind, selector/locator/event/network handle,
  timeout, polling/event strategy, and cancellation policy.
- `BrowserArtifactHandle`: artifact handle, artifact kind, source handle, size
  class, content type, checksum handle, retention, redaction class, and replay
  pointer.
- `BrowserNetworkEvent`: event handle, page/context handle, request/response
  handles, URL handle, method class, status class, resource type, timing class,
  redaction class, and cursor.
- `BrowserConsoleEvent`: event handle, page handle, level, text handle,
  timestamp, source handle, and redaction class.
- `BrowserDialogEvent`: event handle, page handle, dialog kind, message handle,
  default value handle, policy decision, and redaction class.
- `BrowserTraceEvent`: event handle, context/page handle, event kind, timestamp,
  target handle, artifact handle, and redaction class.
- `BrowserStorageHandle`: storage handle, context handle, storage kind,
  origin scope, sensitivity class, retention, and approval state.
- `BrowserSessionSnapshot`: context/page handles, provider capability hash,
  lifecycle state, artifact summaries, event cursors, storage handle summaries,
  resource counters, and replay pointers.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `browser.provider.inspect`
- `browser.context.open`
- `browser.context.close`
- `browser.page.open`
- `browser.page.close`
- `browser.navigate`
- `browser.wait`
- `browser.dom.inspect`
- `browser.locator.resolve`
- `browser.action.perform`
- `browser.evaluate`
- `browser.screenshot`
- `browser.accessibility.inspect`
- `browser.download.manage`
- `browser.upload.manage`
- `browser.events.inspect`
- `browser.storage.manage`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, browser context handle, page/frame handle, origin
  handle, and actor handle when available.
- Context creation requires plan/request separation, isolation mode, storage
  policy, origin allowlist, network policy, artifact policy, resource
  reservation, credential reference, idempotency key, and audit reason.
- Navigation, actions, uploads, downloads, storage export/import, script
  evaluation, cross-origin operations, authenticated pages, form submission, and
  irreversible or external side effects may require approval.
- Screenshots, DOM snapshots, accessibility snapshots, console logs, network
  events, downloads, uploads, and traces require redaction and bounded output.
- Remote browser/grid operations require network permission, provider quota,
  timeout, cancellation, and structured unavailable behavior.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
provider/browser support, context support, page/frame support, locator/action
support, evaluation support, screenshot/accessibility support, download/upload
support, event/network/tracing support, storage support, permission scopes,
policy templates, resource limits, approval rules, provider capability hashes,
health, compatibility, diagnostics, examples, redaction profiles, and
documentation links.

The developer guide at
`docs/developer-packs/developer/browser-automation.md` must cover:

- manifest declaration and optional/required behavior
- provider scopes, context profiles, pages, frames, locators, navigation, waits,
  DOM/query inspection, actions, evaluation, screenshots, accessibility,
  downloads, uploads, events, storage state, traces, cleanup, and provider
  capabilities
- context plan/request lifecycle, action/evaluation planning, origin policy,
  credential/storage policy, network policy, artifact retention, approval,
  quotas, unavailable diagnostics, provider replacement, trace/audit
  interpretation, and conformance tests

Examples must use synthetic pages, URLs, locators, artifacts, and events. They
must not include provider names, real credentials, private cookies, customer
data, local file contents, screenshots, downloads, network payloads, or
website-specific workflows.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `browser_pack_declared`
- `browser_pack_admission_validated`
- `browser_provider_inspected`
- `browser_context_planned`
- `browser_context_open_requested`
- `browser_page_opened`
- `browser_navigation_requested`
- `browser_wait_completed`
- `browser_dom_inspected`
- `browser_locator_resolved`
- `browser_action_planned`
- `browser_action_requested`
- `browser_evaluation_planned`
- `browser_evaluation_requested`
- `browser_screenshot_captured`
- `browser_accessibility_captured`
- `browser_download_managed`
- `browser_upload_managed`
- `browser_events_inspected`
- `browser_storage_state_managed`
- `browser_page_closed`
- `browser_context_closed`
- `browser_pack_policy_decision`
- `browser_pack_service_call_requested`
- `browser_pack_service_call_succeeded`
- `browser_pack_service_call_failed`
- `browser_pack_unavailable`
- `browser_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, context/page
state summaries, origin policy hash, artifact policy hash, command availability,
provider health, resource counters, event cursors, artifact summaries, storage
handle summaries, and sanitized replay pointers. Snapshots must exclude raw
cookies, credentials, storage values, local file contents, raw screenshots, raw
DOM dumps, raw downloads/uploads, network payloads, raw provider payloads,
prompts, manifests, package bytes, private keys, signatures, and unbounded logs.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: browser providers, locator resolvers, actionability validators,
  wait strategies, redaction strategies, artifact retention strategies, storage
  policies, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  network policy, origin policy, credential/storage redaction, artifact
  redaction, and mutation safety wrap service calls.
- **Specification**: admission validates provider scope, context support,
  command availability, permissions, origin policy, artifact policy, provider
  state, quota, and compatibility.
- **Observer**: page lifecycle, console events, network events, dialog events,
  trace events, health, trace, and audit events are subscribable.
- **Memento**: context plans, navigation/action/evaluation plans, page states,
  event cursors, artifact handles, storage handles, snapshots, and replay
  pointers preserve recovery state.
- **Abstract Factory**: concrete browser providers are created only by approved
  runtime-host composition roots.

## Risks And Mitigations

- Risk: pack becomes a Playwright/CDP wrapper. Mitigation: provider-neutral
  context/page/frame/locator/action/artifact DTOs and Strategy adapters.
- Risk: automation leaks cookies, screenshots, DOM, downloads, or network data.
  Mitigation: handles, redaction, bounded artifacts, and strict observability
  exclusions.
- Risk: actions cause irreversible external side effects. Mitigation:
  plan/request split, side-effect classification, approval, and audit.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call browser APIs directly.
- Risk: provider feature differences are hidden. Mitigation: explicit provider
  capability DTO, compatibility hashes, unavailable diagnostics, and conformance
  tests.
