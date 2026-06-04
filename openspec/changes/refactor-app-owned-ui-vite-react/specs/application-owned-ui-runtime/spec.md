## ADDED Requirements

### Requirement: Application-owned bundle source toolchains

Macaca SHALL allow an application-owned web bundle to be authored with an
application-local source toolchain as long as the admitted manifest exposes a
static bundle entry that Macaca can host through the generic UI runtime.

#### Scenario: App-owned React bundle is hosted generically

- **GIVEN** an installed application declares `ui.runtime` as `web_bundle`
- **AND** the application package contains a source toolchain such as Vite,
  React, and TypeScript under its own package directory
- **AND** `ui.entry` resolves to a built static HTML file inside the installed
  application package
- **WHEN** Macaca admits and serves the application UI
- **THEN** Macaca SHALL host the built static entry through the generic
  application UI asset route
- **AND** Macaca SHALL NOT inspect or branch on the source framework,
  application name, workflow name, or business payload
- **AND** application-specific presentation logic SHALL remain inside the
  application package.
