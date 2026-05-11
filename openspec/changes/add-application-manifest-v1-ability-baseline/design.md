## Context

The approved Option E plan defines Application Platform as `Application Package Manifest v1 + Ability Model + SDK Kits + WASM ABI + YAML Adapter + Application Service`. This first slice creates the foundation model only. It does not migrate Web, change YAML behavior, or execute WASM.

## Goals

- Define Application Manifest v1 as the future application package fact source.
- Define Ability Descriptor as the component model inside an application.
- Keep DTOs provider-neutral, serializable, auditable, and stable for SDK/runtime-host/Web/CLI use.
- Keep Kernel free of application/provider/business logic.
- Provide small Specification-style validators that later proposals can reuse.

## Non-Goals

- Do not replace legacy YAML loading in this proposal.
- Do not add real WASM execution.
- Do not move Web raw manifest reads yet.
- Do not add Store, Payment, Web3, Driver, Skill, MCP, LLM, Memory, or Plugin provider implementations.

## Decisions

- Decision: Use Composite for `ApplicationManifestV1 -> AbilityDescriptor -> capability/permission/service declarations`.
  Alternatives considered: keep one flat manifest. Rejected because flat manifests become hard to validate and cannot model multiple app components cleanly.

- Decision: Use Builder-compatible immutable DTOs in `macaca-proto`.
  Rationale: SDK, runtime-host, Web, CLI, and tests must share the same provider-neutral contracts without depending on `macaca-app` internals.

- Decision: Use Specification objects in `macaca-app` for admission.
  Rationale: trace, runtime kind, ability kind, permission, service dependency, and compatibility checks must not be duplicated across shells and hosts.

- Decision: Keep YAML as a compatibility source, not the new fact source.
  Rationale: YAML remains first-class but cannot constrain future WASM/GenUI/headless/hybrid applications.

- Decision: Use Null Object semantics for unsupported runtime declarations in the model.
  Rationale: optional runtime availability must be explicit and auditable rather than panic or silent success.

## Risks / Trade-offs

- Risk: Manifest v1 becomes a huge schema.
  Mitigation: split files by manifest, ability, permissions, services, UI, commerce, and compatibility; keep every Rust file under 500 lines.

- Risk: Ability model is overdesigned.
  Mitigation: only define the minimum ability kinds from the plan and keep execution semantics out of this proposal.

- Risk: DTOs leak business semantics.
  Mitigation: forbid app/workflow/provider/business-name branching and keep fields generic.

## Migration Plan

1. Add protocol DTOs and serialization tests.
2. Add app-layer admission specifications and deterministic projection helpers.
3. Export modules through `lib.rs`.
4. Add tests for all minimum ability kinds and declaration validation.
5. Run dependency/topology gates.

## Trace / Audit

Manifest validation and ability admission reports must include safe ids, runtime kind, ability kind, reason codes, and trace id when supplied. They must never include prompt body, raw full manifest body, raw agent config, secrets, env, API keys, or raw host payloads.
