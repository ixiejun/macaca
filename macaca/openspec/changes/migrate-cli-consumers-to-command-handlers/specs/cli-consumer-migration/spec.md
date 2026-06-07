## ADDED Requirements

### Requirement: Deprecated CLI helper calls are migrated

Upper Rust consumers SHALL NOT call deprecated `macaca-cli` helper functions after the command-handler refactor.

#### Scenario: Deprecated helper scan passes

- **WHEN** the repository scans Rust consumers for deprecated CLI helper calls
- **THEN** no production or test caller uses `macaca_cli::run_kernel`
- **AND** no production or test caller uses `macaca_cli::list_agents`
- **AND** no production or test caller uses `macaca_cli::show_status`
- **AND** no production or test caller uses `macaca_cli::create_kernel`

### Requirement: Process consumers use CLI subcommands

Process-level consumers SHALL continue invoking the `macaca` binary subcommands instead of embedding deprecated Rust helper APIs.

#### Scenario: Development restart uses web command

- **WHEN** the development restart script starts the backend
- **THEN** it invokes the `macaca` binary with the `web` subcommand
- **AND** it does not call deprecated Rust helper APIs

#### Scenario: Systemd service uses web command

- **WHEN** the systemd unit starts Macaca
- **THEN** it invokes `/usr/local/bin/macaca web`
- **AND** it relies on CLI command dispatch for web startup

### Requirement: macaca-cli remains an entry adapter

Core crates SHALL NOT add direct Cargo dependencies on `macaca-cli` as a way to reuse CLI command helpers.

#### Scenario: Cargo dependency guard passes

- **WHEN** Cargo manifests are scanned
- **THEN** no crate other than the workspace root and `macaca-cli` itself declares a dependency on `macaca-cli`
