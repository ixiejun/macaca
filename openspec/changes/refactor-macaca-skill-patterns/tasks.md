## 1. OpenSpec
- [x] 1.1 Add proposal, design, tasks, and delta spec.
- [x] 1.2 Validate with `openspec validate refactor-macaca-skill-patterns --strict`.

## 2. Policy Chain
- [x] 2.1 Add `policy.rs`.
- [x] 2.2 Route runtime filtering through `SkillPolicyChain`.
- [x] 2.3 Add policy tests.

## 3. Source Factory
- [x] 3.1 Add `source.rs`.
- [x] 3.2 Route runtime source construction through `SkillSourceSet`.
- [x] 3.3 Add precedence tests.

## 4. Snapshot / Reload
- [x] 4.1 Add `snapshot.rs`.
- [x] 4.2 Add `SkillRegistry::snapshot` and `SkillRegistry::reload_from_snapshot`.
- [x] 4.3 Mark direct load APIs deprecated only after replacements exist.

## 5. Tool Adapter / Runtime Handle
- [x] 5.1 Add `adapter.rs`.
- [x] 5.2 Add `handle.rs`.
- [x] 5.3 Route `SkillTool` through adapter/proxy.
- [x] 5.4 Extend provisioner to return runtime handles additively.

## 6. Verification
- [x] 6.1 Run `cargo test -p macaca-skill -- --nocapture`.
- [x] 6.2 Run `cargo check -p macaca-skill -p macaca-web -p macaca-runtime-host`.
- [x] 6.3 Run deprecated API containment grep.
- [x] 6.4 Run GitNexus detect changes before commit.
