## ADDED Requirements

### Requirement: Tools SHALL expose canonical command and schema primitives

`macaca-tools` SHALL provide canonical additive-first primitives for tool execution and schema access without breaking existing `Tool` implementations.

#### Scenario: Consumer executes a tool through canonical command context

- **GIVEN** an upper-layer consumer that owns a `macaca-tools::Tool`
- **WHEN** it needs to execute that tool with execution metadata such as trace channels
- **THEN** it SHALL be able to call a canonical command-style execution entry
- **AND** existing `Tool` implementations SHALL continue to work through compatibility adapters

#### Scenario: Consumer reads schema without depending on legacy method name

- **GIVEN** an upper-layer consumer that needs tool definitions
- **WHEN** it reads a tool schema
- **THEN** it SHALL be able to do so through a schema provider primitive
- **AND** existing `parameters_schema()` compatibility behavior SHALL remain available during migration

### Requirement: Tool command middleware SHALL standardize trace emission

`macaca-tools` SHALL provide a standard middleware-based execution chain for tool command hooks such as trace emission.

#### Scenario: Standard trace middleware wraps a tool command

- **GIVEN** a tool command pipeline with the standard trace middleware installed
- **WHEN** a tool is executed through the canonical command path
- **THEN** `tool_call` SHALL be emitted before execution
- **AND** `tool_result` SHALL be emitted after execution
- **AND** concrete business tools SHALL NOT need to handwrite that standard trace sequence

### Requirement: Toolset composition SHALL be provided by macaca-tools

`macaca-tools` SHALL provide a standard composite toolset primitive so upper-layer crates do not need to reimplement ad hoc tool aggregators.

#### Scenario: Upper-layer crate aggregates multiple tool groups

- **GIVEN** a consumer that needs to combine built-in tools, driver tools, skill tools, or test tools
- **WHEN** it constructs a tool catalog
- **THEN** it SHALL be able to use a composite toolset primitive from `macaca-tools`
- **AND** the legacy `ToolSet` query surface SHALL remain available only as a deprecated compatibility layer

### Requirement: Legacy tool contracts SHALL remain as deprecated compatibility wrappers

Existing `macaca-tools` consumer-facing legacy interfaces SHALL remain available during migration, but they SHALL be marked deprecated and canonical consumers SHALL migrate away from them.

#### Scenario: Legacy consumer path still exists during migration

- **GIVEN** an old caller that still reaches a legacy tool contract
- **WHEN** the code compiles
- **THEN** the old API SHALL still exist
- **AND** it SHALL be clearly marked deprecated
- **AND** the canonical replacement SHALL be discoverable in the deprecation guidance

#### Scenario: Framework bridge reads schema through canonical provider

- **GIVEN** a bridge consumer that builds framework tool definitions from a `macaca-tools::Tool`
- **WHEN** it needs a schema for that tool
- **THEN** it SHALL use the canonical schema provider primitive
- **AND** it SHALL NOT depend on the legacy schema method as its primary contract

#### Scenario: Framework bridge executes through canonical command entry

- **GIVEN** a bridge consumer that executes a `macaca-tools::Tool` with streaming or execution metadata
- **WHEN** it invokes the tool
- **THEN** it SHALL use the canonical command-style execution entry
- **AND** compatibility for existing concrete tool implementations SHALL be preserved through additive-first adapters

#### Scenario: Upper-layer consumer avoids deprecated macaca-tools calls

- **GIVEN** an upper-layer crate consumes `macaca-tools`
- **WHEN** it needs tool schema, lookup, definitions, or execution
- **THEN** it SHALL use `ToolSchemaProvider`, `ToolCatalog`, or `ToolCommandExecutor`
- **AND** direct calls to deprecated `macaca-tools` methods SHALL be limited to compatibility adapters inside `macaca-tools` itself or explicitly documented bridge shims

#### Scenario: Non-macaca-tools APIs with matching names remain valid

- **GIVEN** an upper-layer crate calls an API named `tools`, `get_tool`, or `execute`
- **WHEN** that API belongs to another crate's contract such as `macaca-driver::Driver`, `macaca-framework::Toolkit`, or `macaca-framework::ToolHandler`
- **THEN** it SHALL NOT be considered a deprecated `macaca-tools` consumer path
- **AND** the API SHALL remain valid unless that owning crate defines its own migration
