# Design: extend-context-provider-catalog-and-diagnostics

## Patterns

- **Abstract Factory + Builder**: `ContextProviderFactory` remains the plugin boundary; `assemble_context_providers` is a **director** that orders families from config.
- **Decorator**: `VersionedContextProvider` wraps any `ContextProvider` to attach a stable semver string for reporting without changing inner behavior.
- **Strategy**: trust promotion rules are a pluggable list evaluated on each candidate after deny/redact/validate-invariant steps.
- **Anti-corruption / Facade**: external payloads are normalized through `OpaqueExternalPayload` + limits — transports stay outside.
- **Observer (lightweight)**: `ProviderHealthLedger` records last outcomes from `ProviderRuntimeSummary` without embedding prompt text.

## Data flow

1. Config lists `provider_families` (or empty → implicit default order mirroring the historical Web ordering).
2. Assembler resolves each `family_id` against optional `ContextProviderRegistry` plugins, else **builtin** neutral keys (`agent_profile`, `skill_capability`, ...).
3. Facade runs governance + trust, composer, engine.
4. Web updates **health ledger** from `provider_runtime` after assembly.
5. `GET /api/context/provider-runtime` returns descriptors + health (no raw context bodies).

## Coupling rules

- `macaca-runtime` uses `KernelContextAssemblyEnvironment` with **optional** catalogs; unavailable families are skipped with diagnostics, never panics.
- `macaca-web` supplies full `ProviderAssemblyEnvironment` for chat agents.
