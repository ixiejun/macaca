## 1. Context and Audit

- [x] 1.1 Read the CLI refactor plan, consumer migration plan, current CLI code, scripts, systemd unit, and related OpenSpec change.
- [x] 1.2 Audit Rust callers for deprecated CLI helper use.
- [x] 1.3 Audit process consumers for command-line subcommand usage.

## 2. OpenSpec

- [x] 2.1 Create proposal, design, tasks, and delta spec.
- [x] 2.2 Validate `migrate-cli-consumers-to-command-handlers` with `--strict`.

## 3. Consumer Guard

- [x] 3.1 Add a reusable consumer migration guard script.
- [x] 3.2 Ensure the guard fails on deprecated CLI Rust helper calls.
- [x] 3.3 Ensure the guard avoids false positives for domain `list_agents` APIs and documentation.
- [x] 3.4 Ensure no upper crate declares a direct dependency on `macaca-cli`.

## 4. Deprecated Call Migration

- [x] 4.1 Confirm there are no remaining deprecated CLI helper calls outside compatibility definitions/re-exports.
- [x] 4.2 Migrate any discovered deprecated calls to command handlers, non-deprecated execution helpers, or lower-layer crate APIs.
- [x] 4.3 Keep deprecated helper definitions present and marked deprecated.

## 5. Verification

- [x] 5.1 Run `bash -n scripts/check-cli-consumer-migration.sh`.
- [x] 5.2 Run `scripts/check-cli-consumer-migration.sh`.
- [x] 5.3 Run `bash -n scripts/restart-dev.sh`.
- [x] 5.4 Run `bash -n macaca/tests/e2e_project_task.sh`.
- [x] 5.5 Run `cargo fmt --all`.
- [x] 5.6 Run `cargo check -p macaca-cli`.
- [x] 5.7 Run `cargo test -p macaca-cli --lib`.
- [x] 5.8 Run `cargo run -p macaca-cli -- --help`.
- [x] 5.9 Run `cargo run -p macaca-cli -- web --help`.
- [x] 5.10 Run OpenSpec strict validation.
- [x] 5.11 Run GitNexus detect-changes and review affected scope.
