# Change: Add workspace skill projection

## Why

WASM applications can see skill metadata, but model agents may still guess non-existent workspace paths such as `available_skills/<skill>/SKILL.md` when the prompt only exposes global absolute skill locations. Macaca needs a generic OS-level projection so every application receives stable, readable skill paths without hardcoding application names or skill names.

## What Changes

- Discover user-installed skills with Macaca's central store first, then the generic Agent skills directory, then common client skill directories.
- Materialize visible skills into each workspace under `available_skills/<stable-slug>/` before rendering the prompt.
- Render projected `SKILL.md` paths in `<available_skills>` while retaining canonical source paths in the snapshot for audit and path-policy checks.
- Keep application/workspace skills and policy filtering generic; no crypto-specific behavior is introduced.

## Impact

- Affected specs: `agent-skills-runtime`
- Affected code: `macaca/crates/services/macaca-skill/src/source.rs`, `macaca/crates/services/macaca-skill/src/runtime.rs`
- Verification: OpenSpec validation plus targeted `macaca-skill` tests and crate checks.
