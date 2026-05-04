## 1. Preparation

- [x] 1.1 Run GitNexus impact for `AgentBuilder`, `DeclarativeAgent`, `AgentPersona`, and `register_from_config`.
- [x] 1.2 Run baseline `cargo test -p macaca-sdk -- --nocapture`.
- [x] 1.3 Confirm current public exports and direct consumer grep.

## 2. AgentSpec builder product

- [x] 2.1 Add `spec.rs` with `AgentSpec`, `AgentSpecBuilder`, and `TracePolicy`.
- [x] 2.2 Add `AgentBuilder::build_spec`.
- [x] 2.3 Make `AgentBuilder::build` delegate through `AgentSpec`.
- [x] 2.4 Add spec parity tests for manifest, permission, capabilities, LLM options, prompt template, and trace policy.

## 3. Persona prototype

- [x] 3.1 Add `persona_prototype.rs` with `PersonaPrototype` and `PersonaOverrides`.
- [x] 3.2 Add clone/override tests proving the original persona is not mutated.
- [x] 3.3 Export persona prototype primitives.

## 4. SDK validation chain

- [x] 4.1 Add `validation.rs` with `SdkValidator` and `SdkValidationChain`.
- [x] 4.2 Implement validators for current name, permission level, capability name, and temperature rules.
- [x] 4.3 Route `AgentConfig::validate` through `SdkValidationChain::default`.
- [x] 4.4 Add parity tests for all current validation success and failure cases.

## 5. SDK facade and registry adapter

- [x] 5.1 Add `facade.rs` with `MacacaSdk`, `AgentRegistryApi`, and kernel adapter.
- [x] 5.2 Make facade register `AgentSpec` while preserving `Kernel::register_agent` behavior.
- [x] 5.3 Route `register_from_config` and `register_from_file` through `MacacaSdk`.
- [x] 5.4 Mark replaced registry helper and builder compatibility functions deprecated but keep callable.
- [x] 5.5 Add facade registration tests.

## 6. Verification

- [x] 6.1 Run `cargo fmt`.
- [x] 6.2 Run `cargo test -p macaca-sdk -- --nocapture`.
- [x] 6.3 Run `cargo test -p macaca-app -- --nocapture`.
- [x] 6.4 Run `cargo test -p macaca-integration-tests kernel -- --nocapture`.
- [x] 6.5 Run `cargo check -p macaca-sdk -p macaca-app -p macaca-web -p macaca-cli`.
- [x] 6.6 Run `openspec validate refactor-macaca-sdk-patterns --strict`.
- [x] 6.7 Run `gitnexus_detect_changes(scope: "all")`.
