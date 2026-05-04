# Change: Migrate macaca-cli consumers to command handlers

## Why

`macaca-cli` now routes subcommands through Command-pattern handlers and keeps old exported command helper functions as deprecated compatibility surfaces.

Upper consumers must not keep or introduce calls to deprecated CLI Rust helpers. Process-level consumers should continue using the stable `macaca` binary subcommands.

## What Changes

- Add a consumer migration specification for `macaca-cli`.
- Verify Rust consumers do not call deprecated CLI helpers:
  - `run_kernel`
  - `list_agents`
  - `show_status`
  - `create_kernel`
- Keep process consumers such as scripts and systemd services on CLI subcommands.
- Add a repeatable guard script that scans for deprecated CLI helper usage and unintended crate-level dependencies on `macaca-cli`.
- Preserve deprecated helper definitions for migration discovery; do not delete them.

## Impact

- Affected specs: `cli-consumer-migration`
- Affected code: repository validation scripts and OpenSpec documentation
- Compatibility impact: no CLI command or Rust API removal; deprecated helpers remain present but must not be called by migrated consumers.
