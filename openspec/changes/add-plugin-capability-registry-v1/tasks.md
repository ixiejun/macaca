## 1. Preparation

- [x] 1.1 Read the plugin enrichment plan and Plugin Runtime v0 code.
- [x] 1.2 Read Driver/Skill/MCP/Gateway/Memory/Context/LLM service adapter contracts.
- [x] 1.3 Run GitNexus impact before editing existing symbols and report blast radius.

## 2. Protocol Contracts

- [x] 2.1 Add plugin capability descriptor DTOs and capability kinds.
- [x] 2.2 Add capability input/output schema, visibility, permission hints, resource hints, trace schema, and slot metadata.
- [x] 2.3 Add conflict report and ownership DTOs.
- [x] 2.4 Add detailed English comments for descriptor safety and provider-neutral semantics.

## 3. Registry And Discovery

- [x] 3.1 Add contract-first discovery from manifests/repository snapshots.
- [x] 3.2 Add capability ownership index and deterministic query APIs.
- [x] 3.3 Add conflict policy Strategy implementations.
- [x] 3.4 Add activation/deactivation cleanup semantics.

## 4. Built-In Adapter Canonicalization

- [x] 4.1 Canonicalize built-in Driver capability descriptors.
- [x] 4.2 Canonicalize built-in Skill/MCP capability descriptors.
- [x] 4.3 Canonicalize built-in Gateway capability descriptors.
- [x] 4.4 Canonicalize built-in Memory/Context capability descriptors.
- [x] 4.5 Canonicalize built-in LLM Provider/Observability capability descriptors when present.
- [x] 4.6 Mark replaced direct descriptor construction paths as deprecated.

## 5. Capability Call Skeleton

- [x] 5.1 Add provider-neutral capability call envelope.
- [x] 5.2 Require trace context and permission/resource admission before call routing.
- [x] 5.3 Route built-in/descriptor-safe calls or return structured unavailable for unsupported execution.
- [x] 5.4 Emit logs and trace/audit for call attempts, denial, unavailable, and success.

## 6. Verification

- [x] 6.1 Run `openspec validate add-plugin-capability-registry-v1 --strict`.
- [x] 6.2 Run `cargo fmt --all --check`.
- [x] 6.3 Run `cargo check --workspace`.
- [x] 6.4 Run `cargo test -p macaca-proto plugin_capability`.
- [x] 6.5 Run `cargo test -p macaca-kernel plugin_registry`.
- [x] 6.6 Run `cargo test -p macaca-runtime-host plugin_capability`.
- [x] 6.7 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
- [x] 6.8 Run `npx gitnexus detect-changes -r agent` before commit.
