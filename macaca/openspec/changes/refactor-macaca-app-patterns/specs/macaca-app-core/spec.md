## ADDED Requirements

### Requirement: App runtime construction SHALL support additive builder-based assembly

The system SHALL add a builder-based assembly path for `macaca-app` runtime construction while preserving the current `AppRuntime` public startup behavior.

#### Scenario: Existing runtime startup API remains valid

- **GIVEN** existing code starts an application through `AppRuntime::start_app_from_file` or `AppRuntime::start_app`
- **WHEN** the builder-based refactor is applied
- **THEN** the existing startup API SHALL remain available
- **AND** application loading, duplicate detection, agent registration, and loaded app state SHALL remain behaviorally unchanged

#### Scenario: Builder preserves current validation and assembly semantics

- **GIVEN** a manifest and base directory that are valid under the current runtime startup path
- **WHEN** they are assembled through `AppRuntimeBuilder`
- **THEN** the resulting runtime state SHALL be equivalent to the current implementation
- **AND** current error conditions for invalid inputs SHALL remain unchanged

### Requirement: Workflow prompt generation SHALL preserve current default behavior through strategy-based rendering

The system SHALL support strategy-based workflow prompt rendering while preserving the current default prompt output and semantics.

#### Scenario: Default workflow prompt remains compatible

- **GIVEN** an application relies on the current default workflow prompt behavior
- **WHEN** `WorkflowPromptStrategy` and prompt template rendering are introduced
- **THEN** the default rendered prompt SHALL remain behaviorally equivalent to the current prompt
- **AND** coordinator/planner execution expectations SHALL remain unchanged

#### Scenario: Prompt parts are internally structured but externally compatible

- **GIVEN** workflow prompt content is internally split into role, constraints, tools, and handoff sections
- **WHEN** `WorkflowEngine::build_system_prompt` renders the final prompt
- **THEN** the externally visible prompt SHALL remain compatible with current callers
- **AND** no application-specific hardcoding SHALL be introduced outside the strategy input

### Requirement: macaca-app SHALL decouple driver and tool selection rules from hardcoded prompt text

The system SHALL move driver and tool selection rules out of hardcoded default prompt strings and SHALL express them through capability or provider inputs.

#### Scenario: Default prompt does not require a specific hardcoded driver name

- **GIVEN** an application supports multiple drivers or tool providers
- **WHEN** the default workflow prompt is generated after the refactor
- **THEN** the prompt SHALL NOT depend on a single hardcoded driver name for correctness
- **AND** driver/tool selection rules SHALL be derived from runtime capability or provider context

#### Scenario: Existing application behavior remains compatible

- **GIVEN** an existing application such as `FULLSTACK-AUTODEV` or `NEWSROOM-AUTOWRITER`
- **WHEN** workflow prompt generation is refactored
- **THEN** the application SHALL retain the same effective workflow guidance semantics
- **AND** driver/tool visibility SHALL NOT be reduced

### Requirement: macaca-app SHALL support structured application capability composition with legacy-compatible output

The system SHALL support an internal composite representation for application-level capabilities while preserving current externally visible capability behavior.

#### Scenario: Legacy capability output remains unchanged

- **GIVEN** application capabilities are currently represented through manifest-level and agent-level capability lists
- **WHEN** those capabilities are internally stored through a composite structure
- **THEN** the legacy flattened capability output SHALL remain compatible with current callers

#### Scenario: Capability source is preserved internally

- **GIVEN** an application capability originates from manifest, skill, driver, tool policy, or future provider inputs
- **WHEN** that capability is stored in the composite structure
- **THEN** the system SHALL preserve the source information internally
- **AND** the external compatibility surface SHALL remain unchanged

### Requirement: macaca-app refactor SHALL remain additive and application-generic

The system SHALL keep the `macaca-app` refactor additive, trace-safe, and generic across applications.

#### Scenario: Existing applications continue to run without app-specific branches

- **GIVEN** multiple applications are loaded through the same runtime
- **WHEN** the refactor is applied
- **THEN** application differences SHALL be expressed through manifest, capability, tool policy, or prompt strategy inputs
- **AND** the system SHALL NOT introduce hardcoded logic for a single application

#### Scenario: Observability and runtime semantics are preserved

- **GIVEN** application startup and workflow prompt generation currently feed higher-level runtime, task, trace, and resume flows
- **WHEN** `macaca-app` internals are refactored
- **THEN** trace visibility, runtime startup semantics, and application lifecycle behavior SHALL remain unchanged
- **AND** the refactor SHALL NOT reduce Agent OS observability
