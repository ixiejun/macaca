## ADDED Requirements

### Requirement: Escape Hatches SHALL Be Removed Not Only Frozen

After each serviceized capability has a complete service-client replacement, Macaca SHALL remove the corresponding migration-module exemption so that any reference (including pre-existing ones) fails the static escape-hatch gate. The terminal state SHALL contain zero escape-hatch references in production code outside explicit fixtures and tests.

#### Scenario: Replaced escape hatch becomes a hard failure
- **WHEN** a capability's service-client replacement is complete and its migration-module exemption is removed
- **THEN** the escape-hatch gate SHALL fail on any remaining production reference to the old direct field, provider, or runtime
- **AND** the diagnostic SHALL name the file, line, token, and the service-client replacement

#### Scenario: Terminal scan reports zero escape hatches
- **WHEN** the escape-hatch gate runs at terminal state
- **THEN** the production-code occurrence count for `KernelProviderCompat`, `LegacyLlmProvider`, `LegacyToolCatalog`, `LegacyAgentExecutionAdapter`, deprecated `AppState` provider fields, `AppRuntime::start_app`/`start_app_from_file`, `driver_runtime.collect_tools()`, and `mcp_runtime.definitions()` SHALL be zero
- **AND** the only permitted references SHALL be in fixtures or tests

### Requirement: Kernel Provider Compatibility SHALL Be Deleted

The kernel `provider_compat` module and the deprecated `Kernel::new(config, llm, tools)` constructor SHALL be deleted. The only kernel construction path SHALL build the kernel from a service-client `AgentExecutionPort` implementation.

#### Scenario: Kernel has no provider compatibility surface
- **WHEN** `macaca-kernel` is inspected
- **THEN** there SHALL be no `provider_compat` module, no `KernelProviderCompat`, no `LegacyLlmProvider`/`LegacyToolCatalog` re-exports, and no `Kernel::new(config, llm, tools)`
- **AND** `cargo check` SHALL produce no deprecated-item warnings within the kernel crate

### Requirement: Reconciliation Markers Are Removed From Production

Production code SHALL NOT contain the multi-path reconciliation markers used to coordinate legacy and serviceized execution paths.

#### Scenario: Reconciliation markers scan is clean
- **WHEN** the escape-hatch gate scans production sources
- **THEN** it SHALL report zero occurrences of `legacy_unmarked`, `non_authoritative`, `suppress_executor_lifecycle`, and `legacy_chat_main_thread_goal_pause`
- **AND** any `graph_owner` usage SHALL exist only as pure audit metadata, never as a path-discrimination switch
