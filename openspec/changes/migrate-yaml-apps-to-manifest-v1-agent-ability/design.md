## Context

Legacy YAML applications are currently the practical app format. The Application Platform must retain them while moving semantics to Manifest v1 and AgentAbility. This is an adapter migration, not a behavior rewrite.

## Goals

- Preserve existing YAML application behavior.
- Project YAML applications into Manifest v1 and AgentAbility descriptors.
- Make package/ABI descriptor generation prefer the new projection.
- Keep old paths deprecated and searchable for migration/debugging.

## Non-Goals

- Do not remove legacy `AppManifest`.
- Do not change `/api/chat/v2` behavior.
- Do not migrate Web raw manifest reads yet.
- Do not add real WASM or GenUI execution.

## Decisions

- Decision: Use Adapter for YAML projection.
  Rationale: YAML must remain first-class without becoming the new platform schema bottleneck.

- Decision: Use AgentAbility as the semantic target for YAML agents.
  Rationale: YAML app semantics are primarily agent/workflow/tool/context declarations, which map naturally to AgentAbility.

- Decision: Use Memento-style conversion reports.
  Rationale: conversion warnings and inferred defaults must be traceable without changing behavior.

- Decision: Keep legacy startup APIs deprecated rather than removed.
  Rationale: additive migration allows rollback and later grep-based cleanup.

## Risks / Trade-offs

- Risk: Projection changes entry agent semantics.
  Mitigation: add equivalence tests comparing legacy and projected entry data.

- Risk: YAML fields do not map cleanly.
  Mitigation: store legacy-only fields in sanitized compatibility metadata and diagnostics, not raw manifest dumps.

- Risk: Package descriptor changes break Store/compatibility checks.
  Mitigation: verify old and new descriptors preserve key ids, runtime kind, entry, permissions, and capability declarations.

## Migration Plan

1. Add YAML adapter and conversion report.
2. Add AgentAbility projection for inline/file agents and entrypoint/workflow metadata.
3. Update package/ABI descriptor generation to prefer projection.
4. Preserve old direct path as deprecated fallback.
5. Run current YAML app tests plus new equivalence tests.

## Trace / Audit

Conversion reports and logs must include application id, ability id, entry mode, counts, trace id when supplied, and reason codes. They must not include raw prompt body, raw full manifest body, raw agent config, secrets, env, or API keys.
