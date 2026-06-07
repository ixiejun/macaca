## ADDED Requirements

### Requirement: Runtime Loop Template Entrypoints

The runtime SHALL provide non-deprecated execution entrypoints that preserve the current agentic loop behavior while making the loop lifecycle a template primitive.

#### Scenario: Standard loop execution remains compatible

- **WHEN** a caller executes an agentic loop through the new standard entrypoint
- **THEN** the runtime performs the same LLM, tool, permission, context trimming, loop detection, and final response behavior as the legacy standard entrypoint.

### Requirement: Runtime Event Observer Boundary

The runtime SHALL emit execution events through an observer-style boundary instead of requiring loop steps to manipulate raw optional channels directly.

#### Scenario: Evented execution preserves ordering

- **WHEN** a loop iteration produces a tool call
- **THEN** runtime events are emitted in the existing order: thinking, assistant, tool call, optional driver trace, tool result, and completion.

### Requirement: Runtime Tool Command Boundary

The runtime SHALL execute tool calls through a command boundary that preserves timeout, trace forwarding, and error-as-tool-result semantics.

#### Scenario: Tool execution error is returned to the model

- **WHEN** a tool command fails during agentic loop execution
- **THEN** the runtime appends the error as a tool result message and continues according to the model response, rather than failing the whole loop.

### Requirement: Deprecated Compatibility Interfaces

The runtime SHALL keep legacy direct execution methods callable but mark them deprecated after non-deprecated replacements exist.

#### Scenario: Legacy methods remain searchable during migration

- **WHEN** code still calls a deprecated runtime execution method
- **THEN** the compiler surfaces a deprecation warning while the method still delegates to the compatible new implementation.

### Requirement: Application-Agnostic Runtime Primitives

Runtime template primitives SHALL NOT contain application, workflow, provider, driver, or business-specific branching.

#### Scenario: Generic runtime execution

- **WHEN** any application supplies an LLM provider, tool catalog, messages, options, and permission policy
- **THEN** runtime execution proceeds through the same generic primitives without application-specific code paths.
