## 1. Provider Inventory

- [ ] 1.1 Inventory existing file providers and map them to owning services or provider adapters.
- [ ] 1.2 Inventory existing shell providers and map them to owning services or provider adapters.
- [ ] 1.3 Inventory existing browser providers and map them to MCP, plugin, gateway, driver, or unavailable adapters.
- [ ] 1.4 Inventory existing web providers and map them to MCP, plugin, gateway, or unavailable adapters.
- [ ] 1.5 Inventory existing memory and knowledge providers.
- [ ] 1.6 Inventory existing task and scheduler providers.
- [ ] 1.7 Inventory existing skill and MCP providers.
- [ ] 1.8 Inventory existing media and document providers.
- [ ] 1.9 Inventory existing communication and enterprise API providers.
- [ ] 1.10 Inventory existing code execution, computer use, and payment/entitlement providers.
- [ ] 1.11 Record gaps as unavailable providers or extension points, not application-specific shortcuts.

## 2. Family Completion

- [ ] 2.1 Add or adapt descriptors for `file`.
- [ ] 2.2 Add or adapt descriptors for `shell`.
- [ ] 2.3 Add or adapt descriptors for `browser`.
- [ ] 2.4 Add or adapt descriptors for `web`.
- [ ] 2.5 Add or adapt descriptors for `memory`.
- [ ] 2.6 Add or adapt descriptors for `knowledge`.
- [ ] 2.7 Add or adapt descriptors for `task`.
- [ ] 2.8 Add or adapt descriptors for `scheduler`.
- [ ] 2.9 Add or adapt descriptors for `skill`.
- [ ] 2.10 Add or adapt descriptors for `mcp`.
- [ ] 2.11 Add or adapt descriptors for `media`.
- [ ] 2.12 Add or adapt descriptors for `document`.
- [ ] 2.13 Add or adapt descriptors for `communication`.
- [ ] 2.14 Add or adapt descriptors for `enterprise_api`.
- [ ] 2.15 Add or adapt descriptors for `code_execution`.
- [ ] 2.16 Add or adapt descriptors for `computer_use`.
- [ ] 2.17 Add or adapt descriptors for `payment_entitlement`.

## 3. Live Industrial Proof

- [ ] 3.1 Create an application-neutral test manifest using generic tool families and toolsets.
- [ ] 3.2 Run a realistic task that requires research, browser or web access, file operations, shell or code execution, memory recall, document or artifact handling, and scheduled follow-up.
- [ ] 3.3 Capture stable session refs, tool plan aggregate counts, invocation audit refs, artifact refs, and provider health summaries.
- [ ] 3.4 Verify no raw model output or raw provider payload enters the report.
- [ ] 3.5 Verify audit replay and artifact refs.

## 4. Validation

- [ ] 4.1 Add provider family unit tests.
- [ ] 4.2 Add integration tests in `industrial_tool_system.rs`.
- [ ] 4.3 Add boundary tests in `tool_service_boundaries.rs`.
- [ ] 4.4 Run `cargo test -p macaca-integration-tests industrial_tool_system -- --nocapture`.
- [ ] 4.5 Run `cargo test -p macaca-integration-tests tool_service_boundaries -- --nocapture`.
- [ ] 4.6 Run `openspec validate complete-industrial-tool-family-providers --strict`.
- [ ] 4.7 Run `git diff --check`.

## 5. Governance Notes

- [ ] 5.1 Confirm every family is served by an owning service, MCP, plugin, gateway, runtime adapter, or unavailable provider.
- [ ] 5.2 Confirm no family implementation requires application-specific OS code.
- [ ] 5.3 Record GitNexus `CRITICAL` and `HIGH` warnings as notes per user instruction.
