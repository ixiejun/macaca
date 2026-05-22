## 1. Specification

- [x] 1.1 Create OpenSpec proposal, design, tasks, and delta spec for draft-only skill experience evolution.
- [x] 1.2 Validate the OpenSpec change with `openspec validate add-skill-experience-evolution-service --strict`.

## 2. Service Contract

- [x] 2.1 Add sanitized Skill experience proposal DTOs.
- [x] 2.2 Add the `skill.evolution.propose_from_task` command constant.
- [x] 2.3 Extend the Skill service descriptor with evolution capability and permission.

## 3. Runtime Host Provider

- [x] 3.1 Add failing tests for accepted, rejected, and non-mutating proposal behavior.
- [x] 3.2 Add in-memory proposal state to the built-in Skill service provider state helper.
- [x] 3.3 Implement traced proposal command handling and structured logs.

## 4. SDK Facade

- [x] 4.1 Extend `SystemSkillClient` with proposal creation.
- [x] 4.2 Implement unavailable Null Object behavior.
- [x] 4.3 Implement service-backed SDK routing.

## 5. Verification

- [x] 5.1 Run OpenSpec validation.
- [x] 5.2 Run focused runtime-host, macaca-skill, and macaca-sdk checks.
- [x] 5.3 Run `git diff --check` and GitNexus change detection.
