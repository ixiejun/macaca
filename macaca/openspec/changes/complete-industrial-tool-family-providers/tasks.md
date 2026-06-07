## 1. Provider Inventory

- [x] 1.1 Inventory existing file providers and map them to owning services or provider adapters.
- [x] 1.2 Inventory existing shell providers and map them to owning services or provider adapters.
- [x] 1.3 Inventory existing browser providers and map them to MCP, plugin, gateway, driver, or unavailable adapters.
- [x] 1.4 Inventory existing web providers and map them to MCP, plugin, gateway, or unavailable adapters.
- [x] 1.5 Inventory existing memory and knowledge providers.
- [x] 1.6 Inventory existing task and scheduler providers.
- [x] 1.7 Inventory existing skill and MCP providers.
- [x] 1.8 Inventory existing media and document providers.
- [x] 1.9 Inventory existing communication and enterprise API providers.
- [x] 1.10 Inventory existing code execution, computer use, and payment/entitlement providers.
- [x] 1.11 Record gaps as unavailable providers or extension points, not application-specific shortcuts.

## 2. Family Completion

- [x] 2.1 Add or adapt descriptors for `file`.
- [x] 2.2 Add or adapt descriptors for `shell`.
- [x] 2.3 Add or adapt descriptors for `browser`.
- [x] 2.4 Add or adapt descriptors for `web`.
- [x] 2.5 Add or adapt descriptors for `memory`.
- [x] 2.6 Add or adapt descriptors for `knowledge`.
- [x] 2.7 Add or adapt descriptors for `task`.
- [x] 2.8 Add or adapt descriptors for `scheduler`.
- [x] 2.9 Add or adapt descriptors for `skill`.
- [x] 2.10 Add or adapt descriptors for `mcp`.
- [x] 2.11 Add or adapt descriptors for `media`.
- [x] 2.12 Add or adapt descriptors for `document`.
- [x] 2.13 Add or adapt descriptors for `communication`.
- [x] 2.14 Add or adapt descriptors for `enterprise_api`.
- [x] 2.15 Add or adapt descriptors for `code_execution`.
- [x] 2.16 Add or adapt descriptors for `computer_use`.
- [x] 2.17 Add or adapt descriptors for `payment_entitlement`.

## 3. Live Industrial Proof

- [x] 3.1 Create an application-neutral test manifest using generic tool families and toolsets.
- [x] 3.2 Run a realistic task that requires research, browser or web access, file operations, shell or code execution, memory recall, document or artifact handling, and scheduled follow-up.
- [x] 3.3 Capture stable session refs, tool plan aggregate counts, invocation audit refs, artifact refs, and provider health summaries.
- [x] 3.4 Verify no raw model output or raw provider payload enters the report.
- [x] 3.5 Verify audit replay and artifact refs.

## 4. Validation

- [x] 4.1 Add provider family unit tests.
- [x] 4.2 Add integration tests in `industrial_tool_system.rs`.
- [x] 4.3 Add boundary tests in `tool_service_boundaries.rs`.
- [x] 4.4 Run `cargo test -p macaca-integration-tests industrial_tool_system -- --nocapture`.
- [x] 4.5 Run `cargo test -p macaca-integration-tests tool_service_boundaries -- --nocapture`.
- [x] 4.6 Run `openspec validate complete-industrial-tool-family-providers --strict`.
- [x] 4.7 Run `git diff --check`.

## 5. Governance Notes

- [x] 5.1 Confirm every family is served by an owning service, MCP, plugin, gateway, runtime adapter, or unavailable provider.
- [x] 5.2 Confirm no family implementation requires application-specific OS code.
- [x] 5.3 Record GitNexus `CRITICAL` and `HIGH` warnings as notes per user instruction.
