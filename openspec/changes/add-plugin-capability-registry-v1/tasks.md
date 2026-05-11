## 1. Preparation

- [ ] 1.1 Read the plugin enrichment plan and Plugin Runtime v0 code.
- [ ] 1.2 Read Driver/Skill/MCP/Gateway/Memory/Context/LLM service adapter contracts.
- [ ] 1.3 Run GitNexus impact before editing existing symbols and report blast radius.

## 2. Protocol Contracts

- [ ] 2.1 Add plugin capability descriptor DTOs and capability kinds.
- [ ] 2.2 Add capability input/output schema, visibility, permission hints, resource hints, trace schema, and slot metadata.
- [ ] 2.3 Add conflict report and ownership DTOs.
- [ ] 2.4 Add detailed English comments for descriptor safety and provider-neutral semantics.

## 3. Registry And Discovery

- [ ] 3.1 Add contract-first discovery from manifests/repository snapshots.
- [ ] 3.2 Add capability ownership index and deterministic query APIs.
- [ ] 3.3 Add conflict policy Strategy implementations.
- [ ] 3.4 Add activation/deactivation cleanup semantics.

## 4. Built-In Adapter Canonicalization

- [ ] 4.1 Canonicalize built-in Driver capability descriptors.
- [ ] 4.2 Canonicalize built-in Skill/MCP capability descriptors.
- [ ] 4.3 Canonicalize built-in Gateway capability descriptors.
- [ ] 4.4 Canonicalize built-in Memory/Context capability descriptors.
- [ ] 4.5 Canonicalize built-in LLM Provider/Observability capability descriptors when present.
- [ ] 4.6 Mark replaced direct descriptor construction paths as deprecated.

## 5. Capability Call Skeleton

- [ ] 5.1 Add provider-neutral capability call envelope.
- [ ] 5.2 Require trace context and permission/resource admission before call routing.
- [ ] 5.3 Route built-in/descriptor-safe calls or return structured unavailable for unsupported execution.
- [ ] 5.4 Emit logs and trace/audit for call attempts, denial, unavailable, and success.

## 6. Verification

- [ ] 6.1 Run `openspec validate add-plugin-capability-registry-v1 --strict`.
- [ ] 6.2 Run `cargo fmt --all --check`.
- [ ] 6.3 Run `cargo check --workspace`.
- [ ] 6.4 Run `cargo test -p macaca-proto plugin_capability`.
- [ ] 6.5 Run `cargo test -p macaca-kernel plugin_registry`.
- [ ] 6.6 Run `cargo test -p macaca-runtime-host plugin_capability`.
- [ ] 6.7 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
- [ ] 6.8 Run `npx gitnexus detect-changes -r agent` before commit.
