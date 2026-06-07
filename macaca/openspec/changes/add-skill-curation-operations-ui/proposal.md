# Change: Add Skill Curation Operations UI

## Why

Macaca now has service-owned Skill governance, curation dry-run, alias, and
draft experience proposal snapshots, but operators do not have a thin shell
surface to inspect that state.

## What Changes

- Add a Web adapter route that aggregates sanitized Skill operations state
  through the SDK Skill facade.
- Add an application operations tab that displays governance records, dry-run
  recommendations, aliases, and draft proposals.
- Preserve service ownership: Web and frontend do not implement curation,
  lifecycle, merge, archive, alias, or proposal semantics.

## Impact

- Affected specs: `skill-governance-curation`
- Affected code: `macaca-web` route registration, frontend application
  operations dialog, frontend Skill operations types and panel.
