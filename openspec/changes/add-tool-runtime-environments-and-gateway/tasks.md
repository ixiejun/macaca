## 1. Environment Contracts

- [ ] 1.1 Add environment descriptor DTOs.
- [ ] 1.2 Add environment health DTOs.
- [ ] 1.3 Add cleanup DTOs.
- [ ] 1.4 Add artifact root DTOs.
- [ ] 1.5 Add process handle DTOs.
- [ ] 1.6 Add network, filesystem, resource, and secret injection policy DTOs.

## 2. Runtime Host Providers

- [ ] 2.1 Add `tool_service_environment.rs`.
- [ ] 2.2 Add local workspace environment adapter.
- [ ] 2.3 Add local sandbox environment adapter.
- [ ] 2.4 Add provider seams for Docker environment.
- [ ] 2.5 Add provider seams for SSH/remote environment.
- [ ] 2.6 Add provider seams for WASM host import environment.
- [ ] 2.7 Add provider seams for browser sandbox environment.
- [ ] 2.8 Add per-call environment lifecycle support.
- [ ] 2.9 Add session-scoped environment lifecycle support.

## 3. Managed Gateway

- [ ] 3.1 Add `tool_service_gateway.rs`.
- [ ] 3.2 Add gateway descriptor registration.
- [ ] 3.3 Add gateway health and unavailable diagnostics.
- [ ] 3.4 Add gateway metering hooks.
- [ ] 3.5 Add gateway audit hooks.
- [ ] 3.6 Add provider seams for web, browser, media, document, remote sandbox, and enterprise connector gateway routes.

## 4. Validation

- [ ] 4.1 Add environment descriptor serialization tests.
- [ ] 4.2 Add environment health tests.
- [ ] 4.3 Add cleanup tests.
- [ ] 4.4 Add unavailable environment tests.
- [ ] 4.5 Add gateway unavailable tests.
- [ ] 4.6 Add metering/audit hook tests.
- [ ] 4.7 Run `cargo test -p macaca-runtime-host tool_service_environment -- --nocapture`.
- [ ] 4.8 Run `cargo test -p macaca-runtime-host tool_service_gateway -- --nocapture`.
- [ ] 4.9 Run `openspec validate add-tool-runtime-environments-and-gateway --strict`.
- [ ] 4.10 Run `git diff --check`.

## 5. Governance Notes

- [ ] 5.1 Confirm optional providers do not become required base OS dependencies.
- [ ] 5.2 Confirm shell code does not own environment lifecycle or provider routing.
- [ ] 5.3 Record GitNexus `CRITICAL` and `HIGH` warnings as notes per user instruction.
