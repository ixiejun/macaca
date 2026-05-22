## 1. Specification

- [x] 1.1 Create OpenSpec proposal, design, tasks, and delta spec for skill alias resolution.
- [x] 1.2 Validate the OpenSpec change with `openspec validate add-skill-alias-resolution-service --strict`.

## 2. Service Contract

- [x] 2.1 Add alias DTOs for redirect, superseded-by, and absorbed-into records.
- [x] 2.2 Add traced alias upsert, resolve, and snapshot command/result DTOs.
- [x] 2.3 Extend the Skill service descriptor with alias capability and permission.

## 3. Runtime Host Provider

- [x] 3.1 Split mutable governance/alias state into `skill_service_provider_state.rs`.
- [x] 3.2 Implement alias upsert, resolve, and snapshot command handlers.
- [x] 3.3 Keep existing governance dry-run and usage commands behavior-compatible.
- [x] 3.4 Add structured logs for alias command acceptance and completion.

## 4. SDK Facade

- [x] 4.1 Extend `SystemSkillClient` with alias upsert, resolve, and snapshot methods.
- [x] 4.2 Implement unavailable Null Object behavior for alias commands.
- [x] 4.3 Implement service-backed SDK routing for alias commands.

## 5. Verification

- [x] 5.1 Add focused provider tests for alias upsert, resolve, and snapshot.
- [x] 5.2 Run `cargo test -p macaca-runtime-host skill_alias`.
- [x] 5.3 Run `cargo test -p macaca-runtime-host skill_governance`.
- [x] 5.4 Run `cargo test -p macaca-skill skill_descriptor_exports_contract_shape`.
- [x] 5.5 Run `cargo check -p macaca-sdk`.
- [x] 5.6 Run `openspec validate add-skill-alias-resolution-service --strict`, `git diff --check`, and GitNexus change detection.
