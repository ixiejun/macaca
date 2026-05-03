## 1. OpenSpec

- [x] 1.1 Create proposal, design, tasks, and delta spec.
- [x] 1.2 Validate `migrate-runtime-consumers-to-template-primitives` with `openspec validate --strict`.

## 2. Baseline and Impact

- [x] 2.1 Confirm direct Cargo consumers are `macaca-web` and `macaca-integration-tests`.
- [x] 2.2 Scan deprecated runtime execution usage across upper consumers.
- [x] 2.3 Run GitNexus impact for symbols to be edited.
- [x] 2.4 Report impact risk before code edits.

## 3. Integration Consumer Migration

- [x] 3.1 Keep `pipeline_dry_run` on `AgenticLoop::execute_with_events`.
- [x] 3.2 Update stale comments or docs that mention `AgenticLoop::run_with_events`.

## 4. Web Resume Signal Migration

- [x] 4.1 Add a generic web-local `RuntimeResumeSignal`.
- [x] 4.2 Migrate `ActiveSession.resume_tx` to the local signal.
- [x] 4.3 Migrate hook consumer resume sends to the local signal.
- [x] 4.4 Migrate chat orchestrator resume channel construction.
- [x] 4.5 Migrate framework runner middleware matching.
- [x] 4.6 Migrate loop manager goal completion resume sends.
- [x] 4.7 Verify `macaca-web` no longer imports `macaca_runtime::agentic_loop::ResumeReason`.

## 5. Verification

- [x] 5.1 Run `cargo fmt`.
- [x] 5.2 Run `cargo test -p macaca-runtime -- --nocapture`.
- [x] 5.3 Run `cargo test -p macaca-integration-tests pipeline_dry_run -- --nocapture`.
- [x] 5.4 Run `cargo check -p macaca-runtime -p macaca-web -p macaca-integration-tests`.
- [x] 5.5 Run deprecated runtime usage grep for upper consumers.
- [x] 5.6 Run `openspec validate migrate-runtime-consumers-to-template-primitives --strict`.
- [x] 5.7 Run `npx gitnexus detect-changes --repo agent --scope all`.
