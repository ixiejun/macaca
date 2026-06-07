## ADDED Requirements

### Requirement: Macaca SHALL define GenUI schema v0 contracts

Macaca SHALL define provider-neutral GenUI schema v0 contracts for UI intent, component trees, components, events, actions, bindings, permission prompts, approval prompts, trace markers, render surfaces, and structured UI errors.

#### Scenario: UI intent round trips through serde

- **WHEN** a `UiIntent` containing a component tree, bindings, trace markers, and metadata is serialized and deserialized
- **THEN** the decoded intent SHALL preserve application id, session id, surface id, component tree, actions, bindings, trace markers, permission prompts, approval prompts, metadata, and trace context
- **AND** the UI contract SHALL NOT depend on `macaca-web`, frontend implementation details, concrete provider crates, concrete driver implementations, concrete gateway implementations, chain implementations, Store implementation, payment implementation, or business workflows

### Requirement: Macaca SHALL model UI as a controlled component tree

Macaca SHALL model GenUI output as a controlled `UiComponentTree` made of nested `UiComponent` nodes rather than arbitrary executable UI code.

#### Scenario: Supported component tree validates

- **WHEN** an application emits a component tree containing text, markdown, form, button, table, card, list, chart placeholder, trace panel mount, and approval prompt components
- **THEN** GenUI validation SHALL preserve the tree structure
- **AND** the renderer SHALL be able to traverse the tree without application-specific branching

#### Scenario: Unknown component is structured unsupported

- **WHEN** an application emits a future or unknown component kind
- **THEN** parsing SHALL preserve the unknown component as structured data
- **AND** validation/rendering SHALL return or display a structured unsupported component result instead of panicking, hanging, evaluating scripts, or silently dropping the component

### Requirement: Macaca SHALL forbid arbitrary remote UI code execution

GenUI Runtime v0 SHALL NOT execute arbitrary remote JavaScript, inline scripts, remote React components, raw HTML with script behavior, or untrusted UI code.

#### Scenario: Script-like payload is rejected

- **WHEN** a UI component or binding attempts to include script execution payloads
- **THEN** GenUI validation SHALL reject the component with a structured unsafe-ui-payload error
- **AND** the host and renderer SHALL NOT execute the payload

### Requirement: Macaca SHALL expose Application GenUI render capability

Macaca SHALL expose GenUI render capability through the Application Framework and Application ABI host boundary so applications can emit `UiIntent` without accessing presentation shell internals.

#### Scenario: Application emits traced UI intent

- **WHEN** an application emits a UI intent through `macaca:ui/render` with app id, session id, and trace context
- **THEN** the ApplicationHost/GenUI facade SHALL validate the intent
- **AND** it SHALL return a structured render result or unavailable result
- **AND** it SHALL emit logs and trace/audit records for intent receipt, validation, and dispatch

#### Scenario: Missing trace is rejected before render dispatch

- **WHEN** an application emits a UI intent or UI event without trace context
- **THEN** GenUI SHALL reject the request with a structured missing-trace error
- **AND** the UI SHALL NOT be rendered or dispatched as a privileged application event

### Requirement: Macaca SHALL keep Web Shell renderer as a presentation strategy

Macaca SHALL implement the Web Shell GenUI renderer as a presentation strategy that renders schema-defined component trees without owning application, session, task, trace, payment, or package semantics.

#### Scenario: No GenUI surface falls back to chat shell

- **WHEN** an application does not declare or emit a GenUI surface
- **THEN** the frontend SHALL continue to display the existing chat shell and trace UI
- **AND** existing chat, trace, task board, session replay, and recovery behavior SHALL remain available

#### Scenario: GenUI surface mounts without application hardcode

- **WHEN** an application emits a valid GenUI surface
- **THEN** the Web Shell SHALL mount the generic GenUI renderer for that surface
- **AND** the renderer SHALL dispatch by component kind, not by application name, workflow name, provider name, driver name, gateway name, chain name, or business-specific routing

### Requirement: Macaca SHALL decorate application UI with system trace and approval overlays

Macaca SHALL support trace overlays, permission prompts, and approval prompts as system-owned decorations around application UI.

#### Scenario: Trace overlay renders system markers

- **WHEN** a UI component tree contains trace markers
- **THEN** the renderer SHALL display trace overlay information using system-owned UI treatment
- **AND** the trace marker SHALL include app id, session id, trace id, surface id, component id, and timestamp when available

#### Scenario: Approval prompt cannot be forged as system approval

- **WHEN** an application emits an approval prompt component
- **THEN** the shell SHALL render it through a system approval decoration path
- **AND** the prompt SHALL carry policy-ready metadata and trace markers
- **AND** application-provided ordinary components SHALL NOT be able to masquerade as completed system approval

### Requirement: Macaca SHALL convert user interactions into traced UI event commands

Macaca SHALL convert button clicks, form submissions, and supported user interactions into `UiEventCommand` records scoped to application id, session id, surface id, component id, event id, action, payload, and trace context.

#### Scenario: Button click emits traced UI event

- **WHEN** a user clicks a GenUI button
- **THEN** the frontend SHALL create a `UiEventCommand` with application id, session id, surface id, component id, action, payload, and trace context
- **AND** the Web Shell route SHALL persist the UI event to EventLog or equivalent trace path
- **AND** the event SHALL be eligible for application/session handling

#### Scenario: Missing session scope is rejected

- **WHEN** a UI event is posted without session id or application id
- **THEN** the Web Shell SHALL reject the event with a structured missing-scope error
- **AND** it SHALL NOT dispatch the event to the application/session boundary

### Requirement: Macaca SHALL preserve trace and recovery regressions

GenUI Runtime v0 SHALL be implemented additively without regressing real-time trace, historical trace replay, or restart/recovery behavior.

#### Scenario: Route C Phase 06 regression checks pass

- **WHEN** Phase 06 verification runs
- **THEN** the implementation SHALL preserve regression matrix scenarios `RC-TRACE-001`, `RC-TRACE-002`, and `RC-RECOVERY-001`
- **AND** existing YAML applications, chat shell, trace viewer, session logs, task board, driver trace, skill/MCP trace, Web UI, and CLI behavior SHALL continue to compile and run through existing paths until explicitly migrated by later changes

### Requirement: Macaca SHALL log and audit GenUI decisions

Macaca SHALL emit structured logs and presentation-neutral trace/audit records for UI intent emission, validation start/pass/reject, renderer selection, unsupported components, trace overlay application, permission prompts, approval prompts, UI event creation, UI event persistence, dispatch, and rejection.

#### Scenario: Rejected UI operation is auditable

- **WHEN** UI validation, rendering, event persistence, or event dispatch rejects an operation
- **THEN** trace/audit records SHALL include app id, session id when available, trace id when available, surface id, component id when available, event id when available, operation name, structured error code, and policy/permission status when available
- **AND** logs SHALL NOT include secrets, private keys, provider credentials, raw payment credentials, raw encrypted package contents, or unbounded user input

### Requirement: Macaca SHALL document GenUI code with detailed English comments

Macaca SHALL include detailed English comments in new Phase 06 Rust and TypeScript/TSX code explaining UI schema invariants, component tree traversal, renderer strategy, trace/audit behavior, UI event command flow, unsupported component handling, prompt decoration, and explicit non-goals.

#### Scenario: Maintainer can understand GenUI invariants from comments

- **WHEN** a maintainer reads the new GenUI modules
- **THEN** comments SHALL explain what each public type, trait, component, renderer path, event command, validation rule, and unsupported result represents
- **AND** comments SHALL explain how trace, audit, permissions, prompts, renderer boundaries, and non-executable UI invariants are protected
- **AND** comments SHALL explain which future capabilities are intentionally not implemented in Phase 06
