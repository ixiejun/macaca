## 1. Preparation

- [x] 1.1 Run GitNexus impact for `execute_agent` upstream.
- [x] 1.2 Run GitNexus impact for `DeclarativeAgent` upstream.
- [x] 1.3 Confirm current deprecated upper-crate call sites with grep.

## 2. Consumer migration

- [x] 2.1 Replace `AgentServices::empty()` in `macaca-kernel` with `AgentServices::builder().build()`.
- [x] 2.2 Replace `AgentServices::empty()` in `macaca-sdk` tests with `AgentServices::builder().build()`.

## 3. Verification

- [x] 3.1 Run `cargo fmt`.
- [x] 3.2 Run `cargo test -p macaca-sdk declarative_agent -- --nocapture`.
- [x] 3.3 Run `cargo test -p macaca-kernel -- --nocapture`.
- [x] 3.4 Run `cargo check -p macaca-agent -p macaca-framework -p macaca-sdk -p macaca-kernel -p macaca-web`.
- [x] 3.5 Run deprecated-call grep for upper crates.
- [x] 3.6 Run `openspec validate migrate-macaca-agent-consumers-to-primitive-boundaries --strict`.
- [x] 3.7 Run `gitnexus_detect_changes(scope: "all")`.
