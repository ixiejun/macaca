## Context

The command-handler refactor introduced `CliCommandHandler` and concrete handlers for `run`, `agents`, `status`, `version`, and `web`.

The old exported helper functions remain present and deprecated:

- `run_kernel`
- `list_agents`
- `show_status`
- `create_kernel`

Consumer audit found no external Rust crate currently imports `macaca_cli` or calls these deprecated helpers. Existing process consumers use the binary command surface:

- `scripts/restart-dev.sh` starts `macaca web` through `cargo run --bin macaca` or `$MACACA_BIN web`.
- `macaca/deploy/macaca.service` starts `/usr/local/bin/macaca web`.
- `macaca/tests/e2e_project_task.sh` talks to the running HTTP API and does not call CLI Rust APIs.

## Goals

- Lock upper consumers onto command-line subcommands or non-deprecated handler/shared APIs.
- Make deprecated CLI helper use detectable in CI or local verification.
- Keep deprecated helper definitions in place for migration discovery.
- Avoid making core crates depend on `macaca-cli`.
- Preserve current CLI command behavior.

## Non-Goals

- Do not remove deprecated helper functions.
- Do not change shell script or systemd lifecycle behavior.
- Do not introduce `CliRuntimeContext`.
- Do not add a CLI bootstrap facade in this migration slice.
- Do not migrate domain APIs named `list_agents`; only deprecated CLI helpers are in scope.

## Design Decisions

### Process Consumers Stay On CLI Commands

Scripts, services, and manual operator flows should invoke the `macaca` binary:

- `macaca web`
- `macaca run`
- `macaca agents`
- `macaca status`
- `macaca version`

They should not embed `macaca-cli` Rust helper functions.

### Rust Consumers Avoid Deprecated Helpers

Rust code that intentionally embeds CLI behavior must use one of the non-deprecated surfaces:

- command handlers, when command dispatch semantics are needed
- `execute_*` helpers only for internal CLI execution/testing
- lower-layer crates such as `macaca-web`, `macaca-kernel`, or `macaca-gateway` when system functionality rather than CLI behavior is needed

Deprecated helpers remain compatibility-only.

### Guard Script

Add a repository-local shell guard that fails if:

- a Rust/source consumer calls `macaca_cli::run_kernel`, `macaca_cli::list_agents`, `macaca_cli::show_status`, or `macaca_cli::create_kernel`
- a Rust source imports deprecated helpers from `macaca_cli`
- a crate other than the workspace root or `macaca-cli` crate declares a Cargo dependency on `macaca-cli`

The guard should avoid false positives from documentation and from domain APIs such as `Kernel::list_agents`.

## Risks

- Deprecated helper names overlap with non-CLI domain APIs.
- A broad text scan can produce false positives from docs and OpenSpec files.
- Process smoke tests can be expensive because `macaca web` starts a long-running server.
- `macaca-cli` is an entrypoint crate; GitNexus may report high affected flow counts even for behavior-preserving guard work.

## Validation

- `openspec validate migrate-cli-consumers-to-command-handlers --strict`
- `bash -n scripts/check-cli-consumer-migration.sh`
- `scripts/check-cli-consumer-migration.sh`
- `bash -n scripts/restart-dev.sh`
- `bash -n macaca/tests/e2e_project_task.sh`
- `cargo check -p macaca-cli`
- `cargo test -p macaca-cli --lib`
- `cargo run -p macaca-cli -- --help`
- `cargo run -p macaca-cli -- web --help`
