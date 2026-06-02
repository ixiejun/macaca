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

### Requirement: Application surface modes

Macaca SHALL support manifest-declared UI surface modes that separate complete
application workspaces from chat/session shell extensions.

#### Scenario: App declares application center surface

- **GIVEN** an installed application manifest declares `ui.runtime` as
  `web_bundle`
- **AND** it declares `ui.surface.mode` as `application`
- **WHEN** the application is opened in a shell
- **THEN** the shell loads the declared bundle as the center interaction
  surface
- **AND** the shell keeps global navigation, page header, and the universal
  right-side AgentPanel
- **AND** the shell does not render main-thread chat tabs, conversation turns,
  or a bottom chat composer inside that center surface.

#### Scenario: App omits surface declaration

- **GIVEN** an installed application manifest declares `ui.runtime` as
  `web_bundle`
- **AND** it omits `ui.surface`
- **WHEN** the application is opened in a shell
- **THEN** Macaca treats the application as `ui.surface.mode: session`
- **AND** existing chat/session shell behavior remains available.

#### Scenario: App declares session surface

- **GIVEN** an installed application manifest declares `ui.surface.mode` as
  `session`
- **WHEN** the application is opened in a shell
- **THEN** the shell keeps the chat/session workspace as the primary interface
- **AND** any app-owned UI is mounted only through generic session extension
  points, not app-specific shell branches.

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

#### Scenario: UI starts application execution

- **GIVEN** an application manifest declares an execution bridge capability
- **AND** the application UI is loaded in an app-owned surface
- **WHEN** the UI submits a user task through the bridge
- **THEN** the shell starts the generic application execution endpoint for that
  application
- **AND** streams session id, trace, service audit, content, error, and done
  events back to the UI
- **AND** the shell does not interpret application-specific workflow or business
  semantics.

#### Scenario: UI bridge session appears in shell session logs

- **GIVEN** an application-owned UI sends an admitted bridge call with a
  non-empty `session_id`
- **WHEN** the bridge call is routed through the Web shell
- **THEN** the shell SHALL create or update a provider-neutral session
  projection for that application and session
- **AND** `/api/apps/{app_id}/sessions` SHALL include the projected session
  after refresh
- **AND** the projection SHALL use only generic bridge scope and sanitized
  routing metadata
- **AND** the shell SHALL NOT inspect application-specific payload fields,
  workflow names, or business semantics to create the projection.

#### Scenario: Host notifies app-owned UI when the active session changes

- **GIVEN** an application-owned UI bundle is hosted in the Web shell
- **AND** the shell active session changes to a different non-empty session id
- **WHEN** the iframe bridge is available
- **THEN** the shell SHALL send a `macaca.session.changed` message to the
  hosted bundle
- **AND** the message SHALL include only generic session context such as
  `app_id`, `session_id`, `surface_id`, and a trace id
- **AND** the shell SHALL NOT inspect application-specific execution streams,
  workflow names, or business payload fields to render the app-owned UI.

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
