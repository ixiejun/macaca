# Developer Browser Automation Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.developer.browser_automation.v1`. Browser automation must expose browser,
context, page, locator, navigation, action, script, network, storage, download,
capture, tracing, and diagnostics through typed service commands. It must not
hardcode website-specific workflows, browser engines, provider libraries, or raw
debug protocols into OS-layer semantics.

## Source Baseline

- Playwright Browser/Page/Locator and auto-waiting behavior:
  <https://playwright.dev/docs/api/class-page>
  and <https://playwright.dev/>
- Chrome DevTools Protocol:
  <https://chromedevtools.github.io/devtools-protocol/>
- W3C WebDriver BiDi:
  <https://www.w3.org/TR/webdriver-bidi/>
- W3C WebDriver and Selenium WebDriver:
  <https://www.w3.org/TR/webdriver2/>
  and <https://www.selenium.dev/selenium/docs/api/java/org/openqa/selenium/WebDriver.html>

## Supplier API Notes

- Playwright contributes Browser, BrowserContext, Page, Locator, tracing,
  screenshots, downloads, network events, auto-waiting, multiple engines, and
  high-level action semantics. Macaca should model action plans and readiness
  conditions without exposing Playwright-specific locators as stable DTOs.
- CDP contributes Page, Runtime, DOM, Network, Input, Target, Browser, and
  Tracing domains for Chromium-family instrumentation, inspection, debugging,
  profiling, screenshot, and script execution. Macaca should treat CDP as a
  provider protocol behind capability reports, not as the application API.
- WebDriver BiDi contributes bidirectional, event-driven browsing context,
  script, input, network, log, storage, and subscription modules. Macaca should
  model event subscriptions, contexts, and script realms without tying routing
  to a specific browser.
- Selenium/WebDriver contributes local and remote sessions, navigation, element
  interaction, window/session lifecycle, capabilities, and language-neutral W3C
  protocol behavior. Macaca should normalize session, window, and capability
  metadata.

## Macaca-Owned Abstractions

`pack.developer.browser_automation.v1` should define `BrowserSession`,
`BrowserContextHandle`, `BrowserPageHandle`, `BrowserLocator`,
`BrowserNavigation`, `BrowserAction`, `BrowserScript`, `BrowserNetworkEvent`,
`BrowserStorageState`, `BrowserDownload`, `BrowserCaptureArtifact`,
`BrowserTrace`, and `BrowserAutomationProviderCapability`.

The DTOs must carry declared browser capability, context isolation, page/window
identity, selector/locator redaction, action preconditions, timeout/cancellation
state, network event bounds, download artifact handles, trace artifact handles,
provider capability hashes, and replay pointers. Raw CDP/BiDi/WebDriver
payloads, credentials, cookies, tokens, private DOM content, screenshots beyond
policy, and unbounded traces are rejected.

## Explicit Non-Goals

- Do not implement concrete Playwright, Puppeteer, Selenium, CDP, WebDriver,
  browser engine, remote grid, profile store, extension provider, or screenshot
  provider adapters in this research phase.
- Do not define website-specific login, checkout, scraping, testing, crawling,
  browser extension, or application-specific workflows in OS layers.
- Do not expose raw protocol messages, provider locators, browser profile
  paths, extension ids, or provider-specific routing as stable SDK contracts.

## Existing Macaca Platform Inventory

- Generic descriptors, `SystemFacade`, trace-required service calls,
  unavailable/null-object behavior, policy/resource gates, persistence
  snapshots, driver/service patterns, secrets-reference handles, and file
  artifact handles provide reusable substrate.
- Current evidence does not prove browser automation DTOs, providers, SDK
  helpers, WASM ABI metadata, tests, dependency gates, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
