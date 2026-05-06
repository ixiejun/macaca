# Proposal: extend-context-provider-catalog-and-diagnostics

## Why

The baseline `add-context-governance-provider-runtime` change delivered governed provider execution and registry traits, but several OpenSpec tasks remained:

- Configurable provider **family selection and ordering** (2.4).
- **Provider metadata** (version, capability tags, health observability) (2.3).
- **Trust governance** (promotion rules) decoupled from application semantics (3.3).
- **Runtime** visibility into the same catalog abstraction without coupling to Web (4.1).
- **HTTP diagnostics** for operator introspection (4.4).
- **Protocol-agnostic external candidate validation** (5.2).

## What

- Add neutral configuration surfaces on `ContextConfig` for provider families and trust promotions.
- Introduce a **catalog assembler** in `macaca-context` that builds provider lists from config + explicit dependencies (no app names).
- Extend the governance pipeline with an optional **trust policy pass**.
- Add **opaque external payload** validation (anti-corruption) without freezing MCP/HTTP wire formats.
- Expose a **read-only diagnostics** HTTP route backed by in-process descriptors + rolling health.

## Impact

- **Breaking (minor)**: `ContextFacade::assemble_model_context` gains an assembly `policy` struct (replaces bare `Option<ContextGovernanceRuntimeConfig>`).

## Non-goals

- Remote provider RPC protocols or WASM loaders.
- Application-specific routing, agent names, or workflow labels as selection keys.
