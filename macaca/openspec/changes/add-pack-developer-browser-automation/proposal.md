# Change: Add Developer Browser Automation Pack

## Why

Developers need `pack.developer.browser.automation.v1` as an industrial browser
automation capability for isolated browser contexts, pages/tabs, navigation,
DOM/query inspection, locator-based interaction, keyboard/mouse/touch actions,
form input, script evaluation, screenshots, downloads/uploads, console logs,
network/event observation, tracing, accessibility snapshots, session cleanup,
and replay diagnostics. It must not be a thin wrapper around Playwright,
Puppeteer, Selenium, Chrome DevTools Protocol, WebDriver, or one browser engine.

Browser automation can log into accounts, submit forms, trigger payments, reveal
cookies, download private files, upload local files, exfiltrate data through
network requests, and capture sensitive screenshots. Macaca must therefore
expose browser automation only through provider-neutral typed service commands
with session isolation, permission, policy, entitlement, resource, approval,
redaction, trace, audit, health, snapshot, replay, and structured unavailable
behavior.

## Research And Supplier/API Baseline

Official references considered for this pack:

- Playwright documents browser, browser context, page, locator, tracing,
  screenshots, network events, downloads, and action auto-waiting. References:
  https://playwright.dev/docs/api/class-browsercontext and
  https://playwright.dev/docs/api/class-page
- Chrome DevTools Protocol exposes Page, Runtime, DOM, Network, Input, Target,
  Browser, and tracing/debugging domains over a bidirectional protocol.
  Reference: https://chromedevtools.github.io/devtools-protocol/
- W3C WebDriver BiDi defines remote control of user agents through browsing
  contexts, script, input, network, log, storage, and event subscription.
  Reference: https://www.w3.org/TR/webdriver-bidi/
- Selenium WebDriver provides browser automation through W3C WebDriver and
  remote/local browser control. Reference:
  https://www.selenium.dev/documentation/webdriver/

Macaca maps these supplier concepts into provider-neutral browser provider,
browser context, page, frame, locator, navigation request, action plan, input
payload, evaluation plan, screenshot handle, download/upload handle, network
event, console event, trace artifact, accessibility snapshot, session state, and
provider capability DTOs. Concrete browser engines, remote grids, WebDriver
servers, CDP connections, Playwright/Puppeteer adapters, and profile stores stay
behind replaceable providers.

## What Changes

- Add provider-neutral `pack.developer.browser.automation.v1` under the
  `developer` family.
- Define command namespace `browser.*` for:
  - provider and browser capability inspection
  - isolated context/session planning and creation
  - page/tab creation and closure
  - navigation and wait-state handling
  - DOM snapshot/query and locator resolution
  - click/type/fill/select/keyboard/mouse/touch actions
  - script evaluation with sandbox policy
  - screenshot and accessibility snapshot handles
  - download/upload handle operations
  - console, dialog, network, and trace event inspection
  - cookie/storage state handles where policy allows
  - session cleanup and replay diagnostics
- Define DTOs for browser scope, provider capability, context profile, page,
  frame, locator, navigation plan, action plan, input payload, evaluation plan,
  wait condition, screenshot handle, artifact handle, network event, console
  event, dialog event, trace event, storage handle, session snapshot, and
  diagnostics.
- Define permission scopes, policy defaults, origin allowlist strategy, credential
  and storage controls, artifact redaction, resource/entitlement behavior,
  approval rules, SDK discovery, developer documentation, trace/audit events,
  snapshots, replay, and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/developer/browser-automation.md` before implementation
  completion.

## Impact

- Affected specs: `pack-developer-browser-automation`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, browser automation
  service provider or unavailable provider, runtime-host provider adapters,
  artifact/trace/redaction support, replay tests, dependency-boundary gates, and
  developer documentation.
- Non-goals: no concrete Playwright/Puppeteer/Selenium/CDP/WebDriver provider
  implementation in this proposal; no app-specific web workflow or testing
  script; no provider-name, browser-name, website-name, selector-name, or
  workflow-name routing in OS layers; no raw cookies, credentials, local files,
  screenshots, downloads, DOM dumps, network payloads, provider payloads, prompts,
  manifests, or unbounded logs in observability; no SDK/shell/kernel provider
  construction; no fake success when provider, browser support, origin scope,
  entitlement, permission, resource, approval, or host support is absent.
