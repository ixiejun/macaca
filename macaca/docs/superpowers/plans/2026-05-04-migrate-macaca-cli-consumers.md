# macaca-cli Consumer Migration Brainstorm and Plan

Date: 2026-05-04

## 1. Current Code and Consumer Facts

This plan follows:

- `AGENTS.md`
- `macaca/docs/design_patterns.md`
- `macaca/docs/design-pattern-refactor-plans/README.md`
- `macaca/docs/design-pattern-refactor-plans/refactor-order.md`
- `macaca/docs/design-pattern-refactor-plans/macaca-cli.md`
- `docs/superpowers/plans/2026-05-04-refactor-macaca-cli.md`
- `openspec/changes/refactor-cli-command-handlers/*`

The current `macaca-cli` refactor introduced:

- `CliCommandHandler`
- `RunCommandHandler`
- `AgentsCommandHandler`
- `StatusCommandHandler`
- `VersionCommandHandler`
- `WebCommandHandler`
- Non-deprecated shared execution functions:
  - `execute_run_kernel`
  - `execute_list_agents`
  - `execute_show_status`
  - `create_kernel_with_stub_provider`
- Deprecated compatibility functions retained for migration lookup:
  - `run_kernel`
  - `list_agents`
  - `show_status`
  - `create_kernel`
- `#![deny(deprecated)]` in `macaca-cli` library and binary, with a narrow `#[allow(deprecated)]` only for compatibility re-exports.

Consumer scan facts:

- No workspace crate other than the CLI binary itself imports `macaca_cli` as a Rust library consumer.
- `macaca/Cargo.toml` declares `macaca-cli` as a workspace crate and workspace dependency, but no other crate `Cargo.toml` depends on it.
- No Rust production caller outside `macaca-cli` calls `macaca_cli::run_kernel`, `macaca_cli::list_agents`, `macaca_cli::show_status`, or `macaca_cli::create_kernel`.
- The remaining `list_agents()` matches outside CLI are kernel/executor/web/sdk domain APIs, not deprecated CLI APIs.
- Shell/systemd consumers use the command-line surface:
  - `scripts/restart-dev.sh` uses `cargo run --bin macaca -- web --port ...` or `$MACACA_BIN web --port ...`
  - `macaca/deploy/macaca.service` uses `/usr/local/bin/macaca web`
  - `macaca/tests/e2e_project_task.sh` assumes the web API is already running and does not call CLI Rust APIs.
- Documentation references the stable CLI command surface:
  - `macaca run`
  - `macaca web`
  - `macaca agents`
  - `macaca status`
  - `macaca version`

Conclusion:

The immediate consumer migration is mostly a hardening and verification slice, not a broad code migration. The goal is to lock all upper-level entrypoints onto the CLI command surface or new handler/shared APIs, and prevent any new dependency on deprecated CLI helpers.

## 2. Superpowers Brainstorm

### Option A: Minimal consumer hardening and scans

Create an OpenSpec migration change that verifies no upper Rust consumer calls deprecated CLI helpers, keeps scripts/systemd on command-line invocation, and adds scan/test guardrails.

Benefits:

- Matches the actual consumer graph: there are no external Rust callers to migrate.
- Smallest behavior-preserving slice.
- Avoids inventing fake upper-layer code paths just to use the new API.
- Keeps the CLI as a final entry adapter rather than a library facade consumed by core crates.

Risks:

- The change may look small because most migration work is validation and documentation.
- Without a durable guard, future callers could reintroduce `macaca_cli::run_kernel` usage.
- Script/systemd validation is smoke-level unless backend startup is explicitly exercised.

### Option B: Migrate scripts to call a new Rust/API wrapper

Add or expose a structured CLI bootstrap API and rewrite shell/systemd flow to target it indirectly or document it as the canonical entry.

Benefits:

- Pushes all entrypoints toward an explicit facade shape.
- Could prepare future embedding of CLI behavior in other hosts.

Risks:

- Not aligned with current consumers: scripts and systemd should call the binary, not Rust internals.
- Adds abstraction before `CliRuntimeContext` and logging strategy exist.
- Higher risk of changing process lifecycle, environment, cwd, or logging behavior.

### Option C: Move command execution functions out of `commands.rs`

Treat `execute_run_kernel`, `execute_list_agents`, `execute_show_status`, and `create_kernel_with_stub_provider` as the new consumer APIs and migrate any local tests/docs to those names.

Benefits:

- Makes deprecated wrappers clearly compatibility-only.
- Gives a clean Rust surface for any future internal CLI testing.

Risks:

- Still not an upper-consumer migration because no external Rust consumers exist.
- Naming could churn again when `CliRuntimeContext` lands.
- Could overexpose execution helpers that should remain behind handlers.

### Option D: Introduce `CliRuntimeContext` now and migrate consumers to it

Implement the second refactor slice immediately so handlers and any future tests consume a context object.

Benefits:

- Moves toward the documented Builder pattern target.
- Would make future logging/context migration cleaner.

Risks:

- This is implementation work beyond the requested planning step.
- Touches config loading and command execution order.
- Should be a separate proposal after the current command-handler migration is accepted.

### Option E: Update documentation only

Update architecture/deploy docs to mention command handlers and avoid deprecated helpers.

Benefits:

- Low risk.
- Useful for future maintainers.

Risks:

- Insufficient as a migration plan because it does not enforce anything.
- Does not validate scripts, systemd, or Rust scans.

## 3. Recommendation

Choose Option A as the first consumer migration slice, with a small part of Option E.

Rationale:

- `macaca-cli` is a terminal entry crate. Upper consumers should not import it as a core library unless there is a clear embedding use case.
- Current real consumers are process-level command invocations. Those should remain on `macaca web`, `macaca run`, etc., while the binary internally routes through `CliCommandHandler`.
- Deprecated Rust helpers should stay present for migration lookup, but new code should be blocked by `#![deny(deprecated)]` and repository scans.
- Do not introduce `CliRuntimeContext` or a CLI facade in this migration slice; those are follow-up refactor slices.

Recommended change ID:

- `migrate-cli-consumers-to-command-handlers`

## 4. Risk Register

- Risk: There are no external Rust consumers, so a migration proposal may accidentally become documentation-only.
  Control: Include explicit scan guards for deprecated CLI APIs and command smoke checks.

- Risk: `macaca` binary help currently initializes logging before Clap parses help.
  Control: Do not change logging order in the consumer migration slice; only verify existing behavior.

- Risk: `scripts/restart-dev.sh` and `macaca.service` depend on process lifecycle, cwd, env, and port behavior.
  Control: Keep command-line invocation unchanged; validate `web --help` and optionally backend startup if requested.

- Risk: `run` waits for Ctrl-C and may be difficult to smoke test in automation.
  Control: Verify `macaca run --help` if a subcommand help exists; avoid launching blocking `run` during migration unless a timeout harness is explicitly added.

- Risk: Deprecated wrapper names overlap with kernel domain APIs such as `Kernel::list_agents`.
  Control: Use targeted scans for `macaca_cli::...`, `use macaca_cli`, and CLI crate paths rather than broad `list_agents()` scans.

- Risk: GitNexus detect-changes may report HIGH for `main` because CLI is an entry process.
  Control: For implementation, run impact before symbol edits and report the entrypoint blast radius before changing code.

## 5. Write Plan

### Task 1: OpenSpec Proposal

Create `openspec/changes/migrate-cli-consumers-to-command-handlers/`:

- `proposal.md`: explain that real consumers are command-line/process entrypoints and Rust deprecated helper consumers are absent.
- `design.md`: document migration policy:
  - command-line consumers stay on CLI subcommands
  - Rust consumers must use command handlers or non-deprecated execution helpers only when embedding is intentionally needed
  - deprecated helpers remain compatibility-only and must not be used by new code
- `tasks.md`: track scans, script/systemd review, optional docs updates, validation, and GitNexus checks.
- `specs/cli-consumer-migration/spec.md`: add requirements for deprecated API avoidance and command-line entrypoint compatibility.

Validation:

```bash
openspec validate migrate-cli-consumers-to-command-handlers --strict
```

### Task 2: Pre-Edit Impact Analysis

If implementation edits any Rust symbol, run GitNexus impact first. Likely symbols:

```bash
npx gitnexus impact --repo agent "Function:macaca/crates/macaca-cli/src/main.rs:main" --direction upstream
npx gitnexus impact --repo agent "Function:macaca/crates/macaca-cli/src/command_handlers.rs:CliCommandHandler.run" --direction upstream
npx gitnexus impact --repo agent "Function:macaca/crates/macaca-cli/src/commands.rs:run_kernel" --direction upstream
npx gitnexus impact --repo agent "Function:macaca/crates/macaca-cli/src/commands.rs:list_agents" --direction upstream
npx gitnexus impact --repo agent "Function:macaca/crates/macaca-cli/src/commands.rs:show_status" --direction upstream
npx gitnexus impact --repo agent "Function:macaca/crates/macaca-cli/src/commands.rs:create_kernel" --direction upstream
```

If the implementation only changes docs/OpenSpec, symbol impact is not required.

### Task 3: Consumer Audit and Guardrails

Run targeted scans:

```bash
rg -n "macaca_cli::(run_kernel|list_agents|show_status|create_kernel)" macaca/crates macaca/tests scripts docs openspec
rg -n "use macaca_cli::.*(run_kernel|list_agents|show_status|create_kernel)" macaca/crates macaca/tests
rg -n "macaca-cli\\s*=|package\\s*=\\s*\\\"macaca-cli\\\"" macaca -g 'Cargo.toml'
rg -n "cargo run -p macaca-cli|cargo run --bin macaca|\\bmacaca\\s+(run|agents|status|version|web)\\b" scripts macaca/deploy macaca/docs docs openspec
```

Expected result:

- No Rust production or test caller uses deprecated CLI helpers.
- Script/systemd consumers invoke the `macaca` binary subcommands.
- Cargo dependency graph does not introduce a new crate-level dependency on `macaca-cli`.

### Task 4: Optional Documentation Updates

Update only if stale docs imply direct Rust helper usage:

- Prefer CLI command examples for process-level usage.
- Prefer `CliCommandHandler` or specific handler names for internal command dispatch documentation.
- Do not document deprecated helpers as valid extension points.

### Task 5: Validation

Run:

```bash
cargo fmt --all
cargo check -p macaca-cli
cargo test -p macaca-cli --lib
cargo run -p macaca-cli -- --help
cargo run -p macaca-cli -- web --help
openspec validate migrate-cli-consumers-to-command-handlers --strict
```

If scripts are changed, also run:

```bash
bash -n scripts/restart-dev.sh
bash -n macaca/tests/e2e_project_task.sh
```

### Task 6: Scope Detection

Before finishing implementation:

```bash
npx gitnexus detect-changes --repo agent
git status --short
```

Confirm:

- Only expected OpenSpec/docs/script or CLI guard files changed.
- Deprecated CLI helpers are still present.
- New dispatch and upper consumers do not call deprecated helpers.
- No app/workflow/driver/business-specific logic was introduced.

## 6. Deferred Work

### `refactor-cli-runtime-context`

After consumer migration is locked, introduce `CliRuntimeContext` and migrate handlers to receive context instead of directly loading config in command execution helpers.

### `refactor-cli-logging-strategy`

Move logging initialization behind a `LoggingStrategy` while preserving current default file logging behavior.

### `refactor-cli-bootstrap-facade`

Only after context and logging seams are stable, add a CLI bootstrap facade for web/kernel startup. This should not make core crates depend on `macaca-cli`.
