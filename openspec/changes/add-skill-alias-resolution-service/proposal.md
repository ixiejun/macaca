# Change: Add Skill Alias Resolution Service Slice

## Why

Hermes Curator rewrites scheduled references after consolidating skills. Macaca should not rewrite scheduler, task, or context files as a side effect of curation. Instead, skill identity redirects and `absorbed_into` relationships must be represented as service-level alias records so every consumer resolves skill identities through a traced Skill service command.

## What Changes

- Add provider-neutral Skill alias, redirect, and absorbed-into DTOs.
- Extend `service.skill` with traced alias upsert, resolve, and snapshot commands.
- Split runtime-host Skill provider governance logic out of the near-500-line provider file.
- Keep the slice non-destructive: alias records are metadata only and do not patch skill files, task definitions, scheduler jobs, or context snapshots.

## Impact

- Affected specs: `skill-governance-curation`
- Affected code:
  - `macaca/crates/services/macaca-skill/src/governance.rs`
  - `macaca/crates/services/macaca-skill/src/service_contract.rs`
  - `macaca/crates/services/macaca-skill/src/service_adapter.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_state.rs`
  - `macaca/crates/facade/macaca-sdk/src/skill_client.rs`
- Boundary impact:
  - Kernel remains uninvolved except for trace-required service dispatch.
  - Shells and applications consume aliases through SDK/facade clients.
  - Runtime-host remains the built-in provider composition boundary.
