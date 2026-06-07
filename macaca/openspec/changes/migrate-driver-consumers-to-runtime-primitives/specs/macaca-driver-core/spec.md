## ADDED Requirements

### Requirement: Upper-layer driver lifecycle SHALL use a runtime facade

Upper-layer crates SHALL use a `macaca-driver` runtime facade for driver load and reload orchestration instead of manually combining loader, registry, tool counting, and reporting logic.

#### Scenario: Web startup auto-loads drivers

- **GIVEN** driver auto-load is enabled
- **WHEN** the web server starts
- **THEN** it SHALL call the driver runtime facade to load drivers
- **AND** loaded drivers SHALL be registered in the same registry used by agent toolkit construction

#### Scenario: Web reloads drivers

- **GIVEN** a user calls the driver reload endpoint
- **WHEN** drivers are reloaded
- **THEN** the reload SHALL be executed through the driver runtime facade
- **AND** the existing clear-then-load behavior SHALL be preserved

### Requirement: Driver load reporting SHALL be produced by macaca-driver

`macaca-driver` SHALL return load reports that include loaded and failed counts plus per-driver status, error, and tool count data.

#### Scenario: Driver loads successfully

- **GIVEN** a dynamic driver loads successfully
- **WHEN** the runtime reports the load result
- **THEN** the report SHALL include loaded status
- **AND** the report SHALL include the tool count

#### Scenario: Driver load fails

- **GIVEN** a dynamic driver fails to load
- **WHEN** the runtime reports the load result
- **THEN** the report SHALL include failed status
- **AND** the report SHALL include the error string

### Requirement: Driver inventory SHALL be exposed by runtime facade

Upper-layer crates SHALL read driver inventory through the runtime facade rather than manually assembling route view models from registry internals.

#### Scenario: Web lists drivers

- **GIVEN** drivers are registered
- **WHEN** the web driver list endpoint is called
- **THEN** it SHALL obtain driver inventory through the runtime facade
- **AND** the response shape SHALL remain compatible with the existing API

### Requirement: Driver tools SHALL be collected through runtime facade

Upper-layer crates SHALL collect runtime driver tools through the driver runtime facade.

#### Scenario: Agent toolkit includes driver tools

- **GIVEN** drivers are registered
- **WHEN** a framework toolkit is built
- **THEN** driver tools SHALL be collected through the runtime facade
- **AND** the same tools SHALL be registered into the framework toolkit as before

### Requirement: Legacy driver lifecycle APIs SHALL remain as deprecated wrappers

Legacy driver loader and registry lifecycle APIs SHALL remain present for external migration but SHALL not be used by migrated upper-layer code.

#### Scenario: Deprecated API remains available

- **GIVEN** external code still references a legacy driver lifecycle API
- **WHEN** it compiles during migration
- **THEN** the API SHALL still exist
- **AND** deprecation guidance SHALL point to the runtime facade
