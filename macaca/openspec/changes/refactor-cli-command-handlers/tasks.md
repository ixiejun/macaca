## 1. Context and Impact

- [x] 1.1 Read the Superpowers plan, current CLI code, design pattern docs, and OpenSpec instructions.
- [x] 1.2 Run GitNexus impact for CLI symbols that will be edited.
- [x] 1.3 Report blast radius before editing and stop for HIGH/CRITICAL risk if encountered.

## 2. OpenSpec

- [x] 2.1 Create proposal, design, tasks, and delta spec.
- [x] 2.2 Validate `refactor-cli-command-handlers` with `--strict`.

## 3. Command Handler Dispatch

- [x] 3.1 Add `CliCommandHandler` and focused handlers for `run`, `agents`, `status`, `version`, and `web`.
- [x] 3.2 Route `main` command dispatch through handlers.
- [x] 3.3 Preserve existing Clap command definitions and help output.
- [x] 3.4 Preserve web systemd ready/watchdog behavior and web builder delegation.

## 4. Deprecation and Migration Guards

- [x] 4.1 Mark replaced exported command functions as deprecated without deleting them.
- [x] 4.2 Ensure new dispatch does not call deprecated wrappers.
- [x] 4.3 Migrate tests to canonical handler/shared paths where appropriate.
- [x] 4.4 Scan for remaining deprecated CLI calls and document intentional compatibility usage.

## 5. Verification

- [x] 5.1 Run `cargo fmt --all`.
- [x] 5.2 Run `cargo check -p macaca-cli`.
- [x] 5.3 Run `cargo test -p macaca-cli --lib`.
- [x] 5.4 Run `cargo run -p macaca-cli -- --help`.
- [x] 5.5 Run `cargo run -p macaca-cli -- web --help`.
- [x] 5.6 Run `openspec validate refactor-cli-command-handlers --strict`.
- [x] 5.7 Run GitNexus detect-changes and review affected scope.
