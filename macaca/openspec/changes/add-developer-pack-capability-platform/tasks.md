## 1. Research And Governance

- [x] 1.1 Review Macaca architecture governance, microkernel boundaries, serviceization allowlist, design patterns, current OpenSpec specs, and existing domain-pack code.
- [x] 1.2 Research macOS, Windows, Android, and HarmonyOS developer capability models and extract reusable OS-level lessons.
- [x] 1.3 Compare current `DomainPackDefinition`, `DomainPackCatalog`, manifest service contracts, and optional finance pack against the target pack platform.
- [x] 1.4 Decide first public pack taxonomy and identifier/version rules after proposal review.

## 2. OpenSpec And Architecture

- [x] 2.1 Create OpenSpec proposal, design, tasks, and `developer-pack-platform` spec delta.
- [x] 2.2 Review conflicts with active service-runtime, SDK facade, application-framework, package/runtime, plugin, and unified-execution changes.
- [x] 2.3 Update governance docs if ownership language changes beyond existing domain-pack rules.
- [x] 2.4 Record GitNexus HIGH/CRITICAL findings as memo only, per user instruction, before implementation commits.

## 3. Contract Foundation

- [x] 3.1 Extend provider-neutral pack contract DTOs with family, parent, version, stability, service command schemas, permission scopes, policy template, data governance, SDK metadata, diagnostics, and compatibility fields.
- [x] 3.2 Add pack id, family id, version range, and sub-pack relationship validators using the Specification pattern.
- [x] 3.3 Add deterministic effective capability reports with source attribution, unresolved/incompatible packs, required/optional separation, and stable hashes.
- [x] 3.4 Add unit tests for pack expansion, sub-pack inheritance, unresolved required packs, unresolved optional packs, version mismatch, and capability hash stability.

## 4. Application Admission And SDK Discovery

- [x] 4.1 Extend application manifest admission to validate pack declarations without reading provider implementations.
- [x] 4.2 Add SDK pack discovery client methods for listing packs, inspecting pack metadata, resolving application declarations, and explaining unavailable reasons.
- [x] 4.3 Ensure shells use SDK/facade pack discovery only and never import optional package providers.
- [x] 4.4 Add audit-friendly logs for admission, resolution, rejection, and effective capability projection.

## 5. Runtime And Optional Package Registration

- [x] 5.1 Keep runtime-host domain-pack registration generic and descriptor-driven.
- [x] 5.2 Add provider health/snapshot projection fields needed by pack diagnostics without exposing provider payloads.
- [x] 5.3 Add structured unavailable behavior for absent required and optional packs.
- [x] 5.4 Add tests proving base runtime-host contains no business-domain pack implementation and optional packages register only through the descriptor-owned factory seam.

## 6. Initial Pack Catalogs

- [x] 6.1 Define data-only reference metadata for foundation pack capabilities that already exist behind service/facade boundaries.
- [x] 6.2 Define data-only reference metadata for developer pack capabilities that already exist behind service/facade boundaries.
- [x] 6.3 Define data-only reference metadata for knowledge pack capabilities that already exist behind service/facade boundaries.
- [x] 6.4 Normalize finance pack metadata into family/sub-pack form while keeping provider logic in the optional package crate.

## 7. Trace, Audit, And Gates

- [x] 7.1 Emit sanitized pack catalog, resolution, provider registration, policy decision, service call, and unavailable events.
- [x] 7.2 Add replay tests proving pack resolution and service calls are trace-addressable through the canonical service path.
- [x] 7.3 Add static gates for no hardcoded pack business logic in kernel, SDK, shells, and base runtime-host.
- [x] 7.4 Run targeted tests, OpenSpec validation, dependency-boundary gates, and no-direct-provider-call gates before marking implementation complete.
