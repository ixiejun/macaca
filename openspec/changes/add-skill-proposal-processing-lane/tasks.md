## 1. Specification

- [x] 1.1 Create OpenSpec proposal, design, tasks, and delta spec.
- [x] 1.2 Validate the OpenSpec change with `openspec validate add-skill-proposal-processing-lane --strict`.

## 2. Service Contract

- [x] 2.1 Add provider-neutral proposal processing DTOs, states, quality score, duplicate signature, run command, run result, snapshot command, and snapshot result.
- [x] 2.2 Add command constants and export the DTOs through `macaca-skill`.
- [x] 2.3 Extend the Skill service descriptor with processing commands and sanitized permission hints.

## 3. Runtime Host Provider

- [x] 3.1 Add in-memory processing state to the built-in Skill provider Strategy.
- [x] 3.2 Implement deterministic dry-run processing without mutation.
- [x] 3.3 Implement policy-gated apply processing that mutates only processing records.
- [x] 3.4 Implement processing snapshots with backlog and state counters.
- [x] 3.5 Add structured logs for processing run start, completion, rejection, and snapshot emission.

## 4. SDK And Shell Adapters

- [x] 4.1 Add SDK Skill client methods and unavailable Null Object behavior.
- [x] 4.2 Add service-backed SDK command forwarding.
- [x] 4.3 Add Web operations snapshot output for processing state without shell-owned classification.
- [x] 4.4 Defer a Web processing-run route for this slice; apply remains service-owned and is covered by SDK/provider commands until a separate operator workflow needs a route.

## 5. Verification

- [x] 5.1 Add TDD coverage for DTO validation, duplicate signature sanitization, score bounds, dry-run immutability, apply mutation, duplicate suppression, ready-for-materialization marking, missing policy rejection, SDK unavailable behavior, and Web adapter thinness when touched.
- [x] 5.2 Run targeted Rust tests for `macaca-skill`, `macaca-runtime-host`, `macaca-sdk`, and `macaca-web` when touched.
- [x] 5.3 Run `openspec validate add-skill-proposal-processing-lane --strict`.
- [x] 5.4 Run boundary regression checks: `cargo test -p macaca-integration-tests route_c_dependency_boundaries` and `cargo test -p macaca-integration-tests route_c_baseline`.
- [x] 5.5 Run `git diff --check` and GitNexus change detection before completion.

## 6. Root-Cause Hardening

- [x] 6.1 Preserve bounded Agent Execution artifact refs when Web projects execution results into Skill proposal task results.
- [x] 6.2 Expose unprocessed proposals as read-only `Queued` processing records in snapshots so backlog pressure is auditable before apply-mode processing.
- [x] 6.3 Score service-safe artifact evidence from `evidence_ref.artifact_*` metadata without widening the raw metadata allowlist.
- [x] 6.4 Add regression tests for artifact evidence preservation and queued snapshot visibility.
