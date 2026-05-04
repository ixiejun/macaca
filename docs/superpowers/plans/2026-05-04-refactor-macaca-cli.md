# macaca-cli Design Pattern Refactor Brainstorm and Plan

Date: 2026-05-04

## 1. Current Code Facts

This plan follows:

- `AGENTS.md`
- `macaca/docs/design_patterns.md`
- `macaca/docs/design-pattern-refactor-plans/README.md`
- `macaca/docs/design-pattern-refactor-plans/refactor-order.md`
- `macaca/docs/design-pattern-refactor-plans/macaca-cli.md`
- `openspec/AGENTS.md`

`macaca-cli` is phase 5 in the global refactor order: the final delivery entry layer after `macaca-web`. It depends on many lower-level crates and should become a thin, generic entrypoint instead of reassembling system internals.

Current module size snapshot:

- `main.rs`: 80 lines
- `commands.rs`: 212 lines
- `logging.rs`: 333 lines
- `lib.rs`: 9 lines
- `Cargo.toml`: 34 lines

Current behavior snapshot:

- `main.rs` owns Clap parsing, configuration loading, logging initialization, subcommand dispatch, systemd ready/watchdog integration for `web`, and calls `macaca_web::WebServerBuilder`.
- `commands.rs` owns `run_kernel`, `list_agents`, `show_status`, `create_kernel`, `StubLlmProvider`, and private `build_kernel`.
- `logging.rs` owns daily rolling file logging, global guard retention, log cleanup, compression helpers, and `init_logging`.
- `run_kernel`, `list_agents`, and `show_status` all rebuild config, stub LLM, default tools, and kernel separately.
- `Web` is already closer to the desired facade direction because it delegates to `macaca_web::WebServerBuilder`, but systemd lifecycle handling is still embedded in `main`.

Validation already performed before this plan:

- `cargo check -p macaca-cli`
- `cargo test -p macaca-cli --lib`
- `cargo run -p macaca-cli -- --help`
- `cargo run -p macaca-cli -- web --help`

GitNexus note:

- `Function:macaca/crates/macaca-cli/src/main.rs:main` has no upstream callers and is low call-graph risk as an executable entrypoint.
- The GitNexus index appears partially stale for CLI/web calls. Before implementation, refresh or re-check GitNexus and run impact analysis for every Rust symbol that will be edited.

## 2. Superpowers Brainstorm

### Option A: Introduce command handlers first

Define a private or crate-visible `CliCommandHandler` abstraction and move each existing `match` branch into a handler while preserving the current Clap enum and output.

Benefits:

- Directly matches the documented Command pattern target.
- Keeps CLI help output stable because the Clap definitions can remain unchanged.
- Allows one-command-at-a-time migration without forcing logging or web bootstrap changes.
- Makes future deprecation scanning easier because old exported command functions can become wrappers.

Risks:

- If handlers directly call the same global functions and reload config, this only moves code without reducing duplication.
- Async trait object dispatch adds a small amount of complexity; use the existing `async-trait` dependency rather than adding new dependencies.
- Handler construction can become over-abstracted if the initial slice introduces factories too early.

### Option B: Extract `CliRuntimeContext` first

Create a context object for shared CLI runtime inputs such as loaded config, log path/level, current directory, and command-level options.

Benefits:

- Removes repeated `MacacaConfig::load_default()` calls in `main` and `commands`.
- Provides a stable seam for later logging strategy and bootstrap facade.
- Supports tests that inject config without touching process globals.

Risks:

- Touches most command functions at once if introduced before command handlers.
- Context ownership/lifetime choices can prematurely constrain future CLI extensions.
- Config loading behavior must remain identical; changing load timing can alter failure order or log initialization behavior.

### Option C: Introduce `LoggingStrategy`

Wrap the existing `init_logging` behavior behind a strategy interface selected by command or runtime context.

Benefits:

- Aligns with the documented Strategy pattern target.
- Keeps file logging internals isolated from command execution.
- Leaves room for future machine-readable or quiet modes without changing command handlers.

Risks:

- Logging is currently process-global and uses a global guard; strategy changes can accidentally break log flushing.
- Changing initialization order could affect error reporting before tracing is ready.
- This is useful but not the safest first slice because command boundaries are not yet clean.

### Option D: Introduce a CLI bootstrap facade

Create a `MacacaCliBootstrap` or `CliBootstrapFacade` that delegates web/kernel/status/agents startup through lower-level facade APIs.

Benefits:

- Moves CLI toward a thin final entry layer.
- Makes downstream startup composition explicit.
- Fits the final `macaca-cli` role after `macaca-web` and lower crates are stable.

Risks:

- Too broad for the first slice because it depends on command handlers, runtime context, and stable lower-layer facades.
- Could become a new god object if it centralizes all command behavior.
- Web startup and systemd lifecycle are user-facing paths; accidental behavior drift is costly.

### Option E: Split logging internals

Break `logging.rs` into appender, cleanup, compression, and initialization modules.

Benefits:

- Improves local maintainability and test focus.
- Keeps each file comfortably below the 500-line rule.
- Can be behavior-preserving if done mechanically.

Risks:

- Does not address the highest-level architecture issue: command dispatch and runtime assembly.
- Touches process-global logging code and file I/O, which can create subtle test and runtime differences.
- Lower priority because `logging.rs` is not over 500 lines today.

## 3. Recommendation

Choose Option A as the first `macaca-cli` refactor slice, followed by Option B.

Rationale:

- Command handlers are the smallest behavior-preserving step toward a thin CLI entrypoint.
- The current `main` symbol has low upstream blast radius, but high user-facing risk through CLI behavior; preserving Clap definitions and stdout text keeps the first slice reviewable.
- A `CliRuntimeContext` is more valuable after handler boundaries exist, because each handler can accept context without redesigning all command functions in one patch.
- Logging strategy and bootstrap facade should be delayed until command/context seams are stable.

Recommended first change ID:

- `refactor-cli-command-handlers`

Recommended second change ID:

- `refactor-cli-runtime-context`

Recommended later change IDs:

- `refactor-cli-logging-strategy`
- `refactor-cli-bootstrap-facade`

## 4. Risk Register

- Risk: CLI help, stdout text, exit codes, and signal behavior are user-visible contracts.
  Control: Keep Clap enum definitions unchanged in the first slice and run `cargo run -p macaca-cli -- --help`, `cargo run -p macaca-cli -- web --help`, and command smoke tests.

- Risk: `run_kernel` blocks on Ctrl-C and starts gateway adapters.
  Control: Do not change its execution order in the command-handler slice; only wrap invocation.

- Risk: `web` systemd ready/watchdog handling is embedded in `main`.
  Control: Keep systemd handling in its current branch until a dedicated lifecycle helper or facade proposal exists.

- Risk: `commands.rs` uses a stub LLM provider for bootstrapping.
  Control: Do not replace provider selection in the first slice; later context/facade work may move provider construction behind a factory only if behavior stays generic.

- Risk: GitNexus index may be stale.
  Control: Before implementation, run `npx gitnexus analyze` if status/context is stale, then run impact analysis for `main`, `run_kernel`, `list_agents`, `show_status`, `create_kernel`, and any logging symbols being edited.

- Risk: over-design in an entry crate.
  Control: Avoid introducing new dependencies, avoid public APIs unless needed by tests or downstream crates, and keep deprecated wrappers only when there is a migration reason.

## 5. Write Plan

### Task 1: OpenSpec Proposal

Create `openspec/changes/refactor-cli-command-handlers/`:

- `proposal.md`: explain why CLI command dispatch needs a Command-pattern boundary before runtime context and logging strategy changes.
- `design.md`: document handler boundaries, behavior-preservation rules, non-goals, and why `CliRuntimeContext` is deferred to a later slice.
- `tasks.md`: track context checks, GitNexus impact, handler extraction, deprecation marking, validation, and cleanup scans.
- `specs/cli-command-dispatch/spec.md`: add requirements for behavior-preserving command handler dispatch.

Validation:

```bash
openspec validate refactor-cli-command-handlers --strict
```

### Task 2: Pre-Edit Impact Analysis

Run:

```bash
npx gitnexus analyze
npx gitnexus impact --repo agent "Function:macaca/crates/macaca-cli/src/main.rs:main"
npx gitnexus impact --repo agent run_kernel
npx gitnexus impact --repo agent list_agents
npx gitnexus impact --repo agent show_status
npx gitnexus impact --repo agent create_kernel
```

Report direct callers, affected processes, and risk before editing.

### Task 3: Add Command Handler Boundary

Implement a small command-dispatch layer without changing public CLI syntax:

- Keep `Cli` and `Commands` Clap definitions behavior-compatible.
- Add a `CliCommandHandler` trait in a new focused module, for example `command_handlers.rs`.
- Add concrete handlers for `Run`, `Agents`, `Status`, `Version`, and `Web`.
- Keep `Version` output identical.
- Keep `Web` delegation to `macaca_web::WebServerBuilder::new().port(port).serve().await`.
- Keep systemd ready/watchdog behavior unchanged in the web command path.

### Task 4: Preserve Old Interfaces for Migration

If command functions are replaced by handler methods, keep old exported functions for discoverability:

- Mark replaced functions with `#[deprecated(note = "... use CliCommandHandler dispatch ...")]`.
- Do not delete deprecated interfaces in this slice.
- Add internal lint/test coverage that the new dispatch path does not call deprecated wrappers.
- Keep direct calls in existing tests temporarily only if the tests are explicitly validating deprecated migration surfaces; otherwise migrate tests to the new handlers.

### Task 5: Validate Behavior

Run:

```bash
cargo fmt --all
cargo check -p macaca-cli
cargo test -p macaca-cli --lib
cargo run -p macaca-cli -- --help
cargo run -p macaca-cli -- web --help
```

If help output changes, treat it as a regression unless the OpenSpec explicitly approves the change.

### Task 6: Detect Scope and Document Follow-Up

Run:

```bash
npx gitnexus detect-changes --repo agent
rg -n "deprecated|run_kernel\\(|list_agents\\(|show_status\\(|create_kernel\\(" macaca/crates macaca/apps macaca/tests
```

Confirm:

- Only expected CLI dispatch files and OpenSpec proposal files changed.
- No upper-layer consumer still calls newly deprecated CLI interfaces except intentional compatibility tests.
- No workflow, app name, driver name, or business-specific string was introduced.

## 6. Deferred Slices

### `refactor-cli-runtime-context`

After handlers exist, introduce `CliRuntimeContext` with loaded config and runtime inputs. Migrate handlers one by one to accept the context and stop reloading default config where behavior permits.

### `refactor-cli-logging-strategy`

Move process-global logging initialization behind a strategy object, preserving the existing file logger as the default strategy.

### `refactor-cli-bootstrap-facade`

Move web/kernel startup delegation into a CLI bootstrap facade after handler and context seams are stable. This should keep CLI generic and avoid embedding application-specific startup logic.
