# Change: Refactor macaca-cli command dispatch with handlers

## Why

`macaca-cli` is the final local entry layer and currently mixes command parsing, logging setup, command dispatch, systemd lifecycle handling, and lower-layer startup calls in `main.rs` and `commands.rs`.

The first safe refactor slice should introduce a Command-pattern boundary while preserving CLI syntax, help output, stdout text, exit behavior, and existing compatibility entrypoints.

## What Changes

- Add command handler primitives for `run`, `agents`, `status`, `version`, and `web` command execution.
- Keep the existing Clap command definitions behavior-compatible.
- Route new command dispatch through handlers instead of directly calling old exported command functions.
- Mark replaced exported command functions as deprecated compatibility interfaces without deleting them.
- Keep web startup delegated to `macaca_web::WebServerBuilder`.
- Preserve systemd ready/watchdog behavior for the `web` command.

## Impact

- Affected specs: `cli-command-dispatch`
- Affected code: `macaca-cli`
- Compatibility impact: no CLI command removal; old Rust functions remain present and deprecated for migration discovery.
