## 1. Implementation

- [x] 1.1 Add a failing runtime-host test proving proposal-id fallback is no
  longer the preferred materialized Skill name when reusable procedure text is
  available.
- [x] 1.2 Implement deterministic semantic identity derivation inside the
  materialization Builder.
- [x] 1.3 Update generated `SKILL.md` `description` and `When To Use` text to
  expose bounded semantic trigger context.
- [x] 1.4 Add sanitized key-node logs for identity derivation.
- [x] 1.5 Run targeted runtime-host tests and OpenSpec validation.
- [x] 1.6 Require generated packages to follow Skill Creator-compatible
  frontmatter, concise body, provenance, and no-clutter package rules.

## 2. Verification

- [x] 2.1 Run
  `cargo test -p macaca-runtime-host semantic_materialized_skill_identity --manifest-path macaca/Cargo.toml`.
- [x] 2.2 Run
  `cargo test -p macaca-runtime-host proposal_materialization --manifest-path macaca/Cargo.toml`.
- [x] 2.3 Run
  `openspec validate improve-materialized-skill-semantic-identity --strict`.
- [x] 2.4 Run `cargo check -p macaca-runtime-host --manifest-path macaca/Cargo.toml`.
- [x] 2.5 Run `git diff --check`.
