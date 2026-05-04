## ADDED Requirements

### Requirement: Command handler dispatch

`macaca-cli` SHALL route subcommand execution through command handler primitives instead of direct `main.rs` match-branch business logic.

#### Scenario: Run command dispatches through handler

- **WHEN** the `run` subcommand is selected
- **THEN** the CLI dispatch layer invokes the run command handler
- **AND** the run command behavior remains compatible with the previous kernel startup path

#### Scenario: Web command dispatches through handler

- **WHEN** the `web` subcommand is selected with a port
- **THEN** the CLI dispatch layer invokes the web command handler
- **AND** the handler starts the web server through `macaca_web::WebServerBuilder`
- **AND** systemd ready/watchdog behavior is preserved when the systemd feature is enabled

### Requirement: Deprecated compatibility interfaces

`macaca-cli` SHALL keep old exported command helper interfaces available as deprecated compatibility surfaces when they are replaced by handler-based dispatch.

#### Scenario: Old command function remains discoverable

- **WHEN** a caller searches for a replaced exported command function
- **THEN** the function still exists
- **AND** it is marked deprecated with migration guidance
- **AND** canonical command dispatch does not call the deprecated function

### Requirement: CLI behavior preservation

The command-handler refactor SHALL preserve existing CLI syntax, help output, stdout formatting, and non-error exit behavior unless a later proposal explicitly changes them.

#### Scenario: Help output remains compatible

- **WHEN** the user runs `macaca --help` or `macaca web --help`
- **THEN** command names, option names, and help text remain compatible with the previous CLI
