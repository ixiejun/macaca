## Context

S7 introduced Application Service lifecycle, but Web still uses raw manifest reads in compatibility-heavy execution paths. This proposal moves semantic metadata queries behind Application Service sanitized projections while preserving execution behavior.

## Goals

- Provide sanitized application metadata views through Application Service.
- Let Web/CLI/Gateway/framework code use service-backed views for app-owned metadata.
- Keep Application Service from owning task/LLM/tool/driver/skill/MCP execution.
- Preserve current user-visible behavior.

## Non-Goals

- Do not remove all Web `macaca-app` dependency edges in this proposal.
- Do not move framework runner execution into Application Service.
- Do not change YAML behavior.
- Do not expose raw manifests through metadata APIs.

## Decisions

- Decision: Use Projection/Adapter from Manifest v1 and YAML compatibility data to sanitized metadata views.
  Rationale: shells need safe, bounded view models, not raw application config.

- Decision: Use Command for metadata query operations.
  Rationale: queries must carry trace and scope and work over future remote service transport.

- Decision: Use Facade through `SystemApplicationClient`.
  Rationale: Web/CLI/Gateway should not know application projection internals.

- Decision: Keep deprecated fallback.
  Rationale: chat/session/toolkit paths are high-risk and must preserve behavior during migration.

## Risks / Trade-offs

- Risk: Sanitized view omits data required by existing framework paths.
  Mitigation: migrate incrementally with fallback and add targeted equivalence tests.

- Risk: Projection leaks prompt or secrets.
  Mitigation: only expose ids, names, counts, policy flags, declared capability names, safe path metadata, digests, and reason codes.

- Risk: Application Service becomes an execution hub.
  Mitigation: views are metadata only; Task/LLM/Memory/Driver/Skill/MCP execution remains in their services.

## Migration Plan

1. Add metadata query DTOs and sanitized view DTOs.
2. Add `macaca-app` projection from Manifest v1/YAML compatibility data.
3. Extend runtime-host provider and SDK client.
4. Migrate Web call sites service-first with deprecated fallback.
5. Add grep/regression checks for new raw manifest reads.

## Trace / Audit

Every metadata query must include trace and application/session scope where applicable. Logs include service id, operation, trace id, app id, session id, view kind, and counts only.
