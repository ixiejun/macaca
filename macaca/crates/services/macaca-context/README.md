# macaca-context — agent OS prompt composition boundary

Purpose-built crate for provider-neutral **`ContextCandidate` → `ContextPlan` → `CompiledContext`** assembly plus report accounting.

Key rules:

1. Composer entry point: [`ContextFacade`](src/composer/facade.rs) (`assemble_model_context`).
2. No vector-database or MCP transports here — integrations implement narrow traits (`ActiveRecallCapability`, `KnowledgeDigestCapability`, etc.).
3. External/remote payloads must pass [`validate_opaque_external_payload`](src/governance/external_boundary.rs) before becoming [`ContextCandidate`](src/composer/candidate.rs) values.

For architecture history see `openspec/changes/` and `docs/superpowers/plans/`.
