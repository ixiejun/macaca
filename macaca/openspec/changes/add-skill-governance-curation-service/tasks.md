## 1. Specification

- [x] 1.1 Create OpenSpec proposal, design, tasks, and delta spec for the first governance/curation slice.
- [x] 1.2 Validate the OpenSpec change with `openspec validate add-skill-governance-curation-service --strict`.

## 2. Service Contract

- [x] 2.1 Add Skill governance lifecycle, provenance, usage telemetry, and curation dry-run DTOs.
- [x] 2.2 Export the DTOs and command names through `macaca-skill`.
- [x] 2.3 Extend the Skill service descriptor with governance and curation capabilities.

## 3. Runtime Host Provider

- [x] 3.1 Add in-memory governance state to the built-in Skill service provider.
- [x] 3.2 Implement traced usage recording and governance snapshot commands.
- [x] 3.3 Implement deterministic, non-destructive curation dry-run reporting.
- [x] 3.4 Add structured logs at command acceptance and completion points.

## 4. SDK Facade

- [x] 4.1 Extend `SystemSkillClient` with governance and curation methods.
- [x] 4.2 Implement unavailable Null Object behavior for the new methods.
- [x] 4.3 Implement service-backed SDK calls for the new methods.

## 5. Verification

- [x] 5.1 Add focused unit tests for contract descriptor and provider behavior.
- [x] 5.2 Run targeted Rust tests for `macaca-skill`, `macaca-runtime-host`, and `macaca-sdk`.
- [x] 5.3 Run `git diff --check` and GitNexus change detection.
