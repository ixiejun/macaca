## 1. OpenSpec
- [x] 1.1 Move misplaced `refactor-macaca-skill-patterns` change to root `openspec/changes`.
- [x] 1.2 Add migration proposal, design, tasks, and delta spec.
- [x] 1.3 Validate both skill OpenSpec changes with `--strict`.

## 2. macaca-skill consumer facade
- [x] 2.1 Add `SkillSnapshotRequest` and builder.
- [x] 2.2 Add `SkillRuntimeFacade`.
- [x] 2.3 Add `ExecutableSkillToolSet`.
- [x] 2.4 Add tests proving facade behavior matches old direct APIs.

## 3. macaca-web migration
- [x] 3.1 Migrate server startup skill catalog/tool loading.
- [x] 3.2 Migrate framework runner snapshot construction.
- [x] 3.3 Migrate skill MCP snapshot construction.
- [x] 3.4 Migrate app skills status route.

## 4. macaca-app and integration tests
- [x] 4.1 Migrate `SkillLoader` source inventory to canonical source primitives without behavior drift.
- [x] 4.2 Migrate integration tests away from deprecated executable skill APIs.

## 5. Verification
- [x] 5.1 Run `cargo test -p macaca-skill -- --nocapture`.
- [x] 5.2 Run `cargo test -p macaca-app skills::tests -- --nocapture`.
- [x] 5.3 Run `cargo test -p macaca-integration-tests fullstack_autodev -- --nocapture`.
- [x] 5.4 Run `cargo check -p macaca-skill -p macaca-app -p macaca-web -p macaca-runtime-host -p macaca-integration-tests`.
- [x] 5.5 Run deprecated containment grep and verify no upper crate usage remains.
- [x] 5.6 Run GitNexus detect changes before commit.
