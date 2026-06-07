## 1. Environment Contracts

- [x] 1.1 Add environment descriptor DTOs.
- [x] 1.2 Add environment health DTOs.
- [x] 1.3 Add cleanup DTOs.
- [x] 1.4 Add artifact root DTOs.
- [x] 1.5 Add process handle DTOs.
- [x] 1.6 Add network, filesystem, resource, and secret injection policy DTOs.

## 2. Runtime Host Providers

- [x] 2.1 Add `tool_service_environment.rs`.
- [x] 2.2 Add local workspace environment adapter.
- [x] 2.3 Add local sandbox environment adapter.
- [x] 2.4 Add provider seams for Docker environment.
- [x] 2.5 Add provider seams for SSH/remote environment.
- [x] 2.6 Add provider seams for WASM host import environment.
- [x] 2.7 Add provider seams for browser sandbox environment.
- [x] 2.8 Add per-call environment lifecycle support.
- [x] 2.9 Add session-scoped environment lifecycle support.

## 3. Managed Gateway

- [x] 3.1 Add `tool_service_gateway.rs`.
- [x] 3.2 Add gateway descriptor registration.
- [x] 3.3 Add gateway health and unavailable diagnostics.
- [x] 3.4 Add gateway metering hooks.
- [x] 3.5 Add gateway audit hooks.
- [x] 3.6 Add provider seams for web, browser, media, document, remote sandbox, and enterprise connector gateway routes.

## 4. Validation

- [x] 4.1 Add environment descriptor serialization tests.
- [x] 4.2 Add environment health tests.
- [x] 4.3 Add cleanup tests.
- [x] 4.4 Add unavailable environment tests.
- [x] 4.5 Add gateway unavailable tests.
- [x] 4.6 Add metering/audit hook tests.
- [x] 4.7 Run `cargo test -p macaca-runtime-host tool_service_environment -- --nocapture`.
- [x] 4.8 Run `cargo test -p macaca-runtime-host tool_service_gateway -- --nocapture`.
- [x] 4.9 Run `openspec validate add-tool-runtime-environments-and-gateway --strict`.
- [x] 4.10 Run `git diff --check`.

## 5. Governance Notes

- [x] 5.1 Confirm optional providers do not become required base OS dependencies.
- [x] 5.2 Confirm shell code does not own environment lifecycle or provider routing.
- [x] 5.3 Record GitNexus `CRITICAL` and `HIGH` warnings as notes per user instruction.

GitNexus note: impact analysis was attempted for `tool_service_descriptor`,
`ToolPlanningService`, `ToolSystemServiceProvider`, and the relevant `lib.rs`
paths in the indexed `agent` and `agent-macaca-phase07` repos. GitNexus returned
target-not-found results for these newer tool-service symbols, so no
`CRITICAL`/`HIGH` blast-radius warning was available to record for this slice.
