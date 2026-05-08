## ADDED Requirements

### Requirement: Developer ecosystem documentation

The system SHALL provide developer documentation for application, plugin, GenUI, Store submission, Web3, and EVM/DApp package development. The documentation SHALL describe package metadata, permissions, trace/audit expectations, debugging, certification commands, and unavailable-safe behavior.

#### Scenario: Developer reads application documentation

- **WHEN** a developer opens the application development guide
- **THEN** the guide explains YAML app development, WASM-stub package metadata, package manifest fields, permission declarations, trace requirements, and certification commands

#### Scenario: Developer reads plugin documentation

- **WHEN** a developer opens the plugin development guide
- **THEN** the guide explains gateway plugin and driver plugin metadata, lifecycle, capabilities, permissions, trace requirements, and service registration boundaries

#### Scenario: Developer reads Store and optional module documentation

- **WHEN** a developer opens Store, Web3, or DApp documentation
- **THEN** the guide explains entitlement states, optional Web3/EVM unavailable behavior, trace requirements, and certification expectations without requiring real payment or blockchain execution

### Requirement: SDK package fixtures

The system SHALL provide SDK package fixtures for YAML application, WASM-stub application, GenUI application, gateway plugin, driver plugin, paid skill, optional Web3 application, and optional EVM/DApp package classes.

#### Scenario: Certification reads SDK fixtures

- **WHEN** package certification tests load SDK fixtures
- **THEN** each fixture produces package metadata that can be evaluated by the compatibility checker without external network, real LLM, real payment, real Store, real browser, real blockchain node, or real EVM runtime

#### Scenario: Fixture is generic

- **WHEN** a hardcode scan is run over SDK fixtures
- **THEN** the fixtures do not hardcode application names, provider names, driver names, gateway names, model names, chain names, or business-specific routing

### Requirement: Package compatibility checker

The system SHALL provide an additive compatibility checker that evaluates package descriptors against host compatibility context and returns a structured report with status, diagnostics, trace/audit events, optional module status, and upgrade notes.

#### Scenario: Package is compatible

- **WHEN** a package declares valid manifest version, runtime kind, ABI version, permissions, required services, and trace metadata
- **THEN** the checker returns `compatible` with no error diagnostics and emits checker start, rule pass, and final decision trace records

#### Scenario: Package is compatible with warnings

- **WHEN** a package declares missing optional services, unavailable optional Web3/EVM modules, or forward-compatible metadata that can be safely ignored
- **THEN** the checker returns `compatible_with_warnings` with structured warning diagnostics and does not reject the package

#### Scenario: Package is incompatible

- **WHEN** a package is missing runtime kind, requires unsupported ABI, lacks required services, or has invalid permission metadata
- **THEN** the checker returns `incompatible` with stable diagnostic codes, field paths, and actionable messages

### Requirement: Traceable and auditable checker execution

The system SHALL emit presentation-neutral trace/audit records and structured logs for compatibility checker start, rule start, pass, warning, failure, optional module degradation, entitlement diagnostics, upgrade diagnostics, and final status.

#### Scenario: Checker evaluates a package

- **WHEN** the compatibility checker runs
- **THEN** every rule decision is represented in the checker report and key execution nodes are logged with structured `tracing` fields

#### Scenario: Checker rejects a package

- **WHEN** a compatibility rule rejects a package
- **THEN** the report includes the rejecting rule, diagnostic code, severity, field path, message, and trace/audit event without panicking or hanging

### Requirement: Certification test harness

The system SHALL provide package certification tests for all required Phase 13 developer paths. The certification tests SHALL use reusable package-class flows rather than one-off file-existence checks.

#### Scenario: YAML and WASM packages are certified

- **WHEN** certification tests run for YAML and WASM-stub application fixtures
- **THEN** YAML package certification passes and WASM-stub certification reports metadata compatibility with structured execution-unavailable status

#### Scenario: GenUI and plugin packages are certified

- **WHEN** certification tests run for GenUI, gateway plugin, and driver plugin fixtures
- **THEN** the tests verify schema, trace, capability, permission, and lifecycle metadata

#### Scenario: Commercial and optional module packages are certified

- **WHEN** certification tests run for paid skill, optional Web3 app, and optional EVM/DApp fixtures
- **THEN** the tests verify entitlement allow/deny diagnostics and unavailable-safe optional module behavior

### Requirement: Upgrade compatibility policy

The system SHALL define and enforce compatibility rules for OS version, Application ABI version, package manifest version, and runtime kind. The checker SHALL distinguish compatible, compatible-with-warning, and incompatible upgrade states.

#### Scenario: Package targets current versions

- **WHEN** a package targets the host-supported manifest and ABI versions
- **THEN** the checker returns compatible unless another rule rejects the package

#### Scenario: Package targets a safe future version

- **WHEN** a package declares a future version that the host can safely inspect but not fully understand
- **THEN** the checker returns compatible-with-warnings with an upgrade diagnostic

#### Scenario: Package targets an incompatible version

- **WHEN** a package requires an ABI, runtime, or manifest version that the host cannot support
- **THEN** the checker returns incompatible with an actionable upgrade diagnostic

### Requirement: Route C regression protection

The system SHALL preserve all Route C regression matrix scenarios while adding ecosystem hardening. The implementation SHALL remain additive-first and SHALL NOT require existing YAML applications or `/api/chat/v2` flows to migrate.

#### Scenario: Route C baseline still passes

- **WHEN** Phase 13 implementation is complete
- **THEN** the Route C baseline integration test passes and current YAML application, trace, task board, resume, driver, skill/MCP, and optional Web3/EVM unavailable-safe paths remain intact

#### Scenario: No application-specific hardcoding is introduced

- **WHEN** hardcode scans run over new ecosystem hardening code, examples, docs, and tests
- **THEN** no application-specific, provider-specific, driver-specific, gateway-specific, model-specific, chain-specific, or business-specific routing is present outside clearly generic fixture metadata
