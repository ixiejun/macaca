## ADDED Requirements

### Requirement: Application-owned web UI bundles

Macaca SHALL support application manifests that declare an app-owned web UI
bundle as the primary interactive UI surface.

#### Scenario: App declares a web bundle UI

- **GIVEN** an installed application manifest declares `ui.runtime` as
  `web_bundle`
- **AND** `ui.entry` resolves to a file inside the application package
- **WHEN** the application is opened in a shell
- **THEN** the shell loads the declared bundle through a generic sandboxed host
  surface
- **AND** the shell does not branch on application id, service id, domain pack,
  workflow name, or business data.

### Requirement: UI bundle sandbox admission

Macaca SHALL validate UI bundle declarations before exposing them to shells.

#### Scenario: UI entry escapes package root

- **GIVEN** an application manifest declares `ui.entry` as a path outside the
  installed application package
- **WHEN** application admission runs
- **THEN** Macaca rejects the UI declaration
- **AND** records an audit event with a stable reason code.

### Requirement: Capability-governed UI bridge

Macaca SHALL require app-owned UI bundles to call host services through a
capability-governed bridge.

#### Scenario: UI calls declared bridge capability

- **GIVEN** an application manifest declares `service.call` in
  `ui.bridge.required`
- **AND** the UI bundle has completed a valid bridge handshake
- **WHEN** the UI sends a bridge call for `service.call`
- **THEN** Macaca evaluates bridge policy
- **AND** routes the accepted call through the generic service router
- **AND** records the policy decision and route result in the audit chain.

#### Scenario: UI calls undeclared bridge capability

- **GIVEN** an application manifest does not declare `storage.kv`
- **WHEN** the UI sends a bridge call for `storage.kv`
- **THEN** Macaca denies the call before routing
- **AND** returns a structured policy error to the UI
- **AND** records the denial in the audit chain.

### Requirement: Optional developer UI Kit

Macaca MAY provide a UI Kit and SDK for application developers, but shells SHALL
not require applications to use Macaca UI components.

#### Scenario: App uses its own React components

- **GIVEN** an application UI bundle is built with React and a custom design
  system
- **WHEN** the bundle is admitted and loaded
- **THEN** Macaca hosts it through the same generic UI runtime
- **AND** does not require `@macaca/ui` components to be present.

### Requirement: Presentation schema fallback

Macaca SHALL treat declarative presentation schema as fallback rendering, not as
the primary application UI runtime.

#### Scenario: UI bundle fails to load

- **GIVEN** an application declares `ui.runtime: web_bundle`
- **AND** the shell cannot load the declared bundle
- **WHEN** the shell renders the failure state
- **THEN** it may use a generic fallback renderer for the error and audit data
- **AND** it records the fallback reason in audit metadata.

