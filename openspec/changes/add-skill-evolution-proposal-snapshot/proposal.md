# Change: Add Skill Evolution Proposal Snapshot

## Why

The Skill service can create draft-only experience proposals, but later curation, approval, review, and audit paths need a service-owned way to inspect those proposals without reading provider internals or mutating skill files.

## What Changes

- Add a traced `skill.evolution.snapshot` Skill service command.
- Return sanitized, sorted draft proposal records with `mutated = false`.
- Expose the command through the SDK Skill client with unavailable Null Object behavior.
- Keep active skill files, governance records, aliases, and catalogs unchanged.

## Impact

- Affected specs: `skill-governance-curation`
- Affected code: `macaca-skill` evolution/service contracts, `macaca-runtime-host` Skill provider, `macaca-sdk` Skill client
