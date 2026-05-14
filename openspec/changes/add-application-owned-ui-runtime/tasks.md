# Tasks

- [ ] Add manifest model types for `ui`, sandbox, bridge, and theme declarations.
- [ ] Parse and project `ui` from YAML manifest v1 into the canonical application manifest.
- [ ] Add admission validation for UI entry paths, asset paths, sandbox mode, CSP mode, and bridge capabilities.
- [ ] Expose UI metadata through shell-facing app/session API responses without leaking runtime internals.
- [ ] Add Web shell iframe host component for `runtime: web_bundle` applications.
- [ ] Add frontend bridge runtime for handshake, call, result correlation, timeout, and structured errors.
- [ ] Add backend bridge route that enforces manifest-declared UI bridge capabilities before routing calls.
- [ ] Route admitted bridge `service.call` requests through the generic service router.
- [ ] Emit audit events for UI admission, surface lifecycle, bridge handshake, bridge policy decisions, and bridge route results.
- [ ] Add minimal `@macaca/app-sdk` local package or module with raw bridge client and React hooks.
- [ ] Add optional `@macaca/ui` package direction only if needed for the first demo; do not make app usage mandatory.
- [ ] Add app-owned React UI bundle to `/Users/quantum/Code/dev/wasm-stock-agent-app`.
- [ ] Update stock app manifest with `ui.runtime: web_bundle` and declared bridge capabilities.
- [ ] Verify Web UI loads the stock app UI from the app package and no frontend code branches on stock, finance, ticker, or service names.
- [ ] Run Rust, frontend, and stock app tests.

