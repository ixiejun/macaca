## 1. Specification

- [x] 1.1 Create OpenSpec proposal, design, tasks, and delta spec for the read-only proposal snapshot.
- [x] 1.2 Validate the OpenSpec change with `openspec validate add-skill-evolution-proposal-snapshot --strict`.

## 2. Service Contract

- [x] 2.1 Add Skill evolution proposal snapshot DTOs.
- [x] 2.2 Add the `skill.evolution.snapshot` command constant.
- [x] 2.3 Preserve the existing Skill evolution capability and permission without changing active semantics.

## 3. Runtime Host Provider

- [x] 3.1 Add a failing provider test for proposal snapshot after proposal creation.
- [x] 3.2 Add in-memory proposal snapshot support to the built-in Skill provider state helper.
- [x] 3.3 Implement traced snapshot command handling and structured logs.

## 4. SDK Facade

- [x] 4.1 Extend `SystemSkillClient` with proposal snapshot.
- [x] 4.2 Implement unavailable Null Object behavior.
- [x] 4.3 Implement service-backed SDK routing.

## 5. Verification

- [x] 5.1 Run OpenSpec validation.
- [x] 5.2 Run focused runtime-host, macaca-skill, and macaca-sdk checks.
- [x] 5.3 Run `git diff --check` and GitNexus change detection.
