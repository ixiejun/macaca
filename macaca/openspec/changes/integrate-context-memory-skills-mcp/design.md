# Design: Umbrella integrate-context-memory-skills-mcp

## Architectural stance

Follow the **recommended hybrid** from the 2026-05-05 plan:

1. Composer foundation (`macaca-context` candidates + facade + composer).
2. Profile files as stable/dynamic gated providers.
3. Active recall strictly via **`ActiveRecallCapability`** / **`MemorySourceProvider`** (no vector DB imports in context).
4. Skills + MCP expose **compact capability snapshots** (`CapabilityCandidate` alias = `ContextCandidate` with capability kind).
5. Knowledge digest bridges governance outputs; tombstones align **digest evidence** and **workspace recall** where applicable.

## Mapped OpenSpec deltas (historic)

| Plan phase | Representative change dirs |
|------------|---------------------------|
| Composer foundation | `add-context-composer-foundation`, `add-context-governance-provider-runtime` |
| Profile | `add-agent-profile-context-provider` |
| Active vector recall | `add-active-vector-memory-context`, `add-memory-active-recall-integration` |
| Skills/MCP capability | `add-skills-mcp-capability-context` |
| Digest / governance overlap | `add-knowledge-digest-context-provider`, `add-memory-governance-knowledge-layer` |
| Runtime web bridge | `complete-context-engine-runtime-phases`, `extend-context-provider-catalog-and-diagnostics` |

## New umbrella decisions (this rollout)

### Profile Markdown

Strip well-formed YAML frontmatter; scan for NUL and optional max line budgets **without** coupling to persona application names.

### Tombstones

Recall uses **`TombstoneIndex`** snapshots (merged via `MergedTombstoneIndex` when multiple sources exist). Fail-open on snapshot errors (warn + include rows) to avoid blocking model calls.

### Composer fingerprints

`ComposerPlanSummary` carries **stable** vs **dynamic** SHA-256 digests over **selected** composer candidates (post-budget), split by `ContextCacheClass::Stable` vs dynamic/unknown.

### External providers

Remote transports remain **out of `macaca-context`**; inputs must pass `OpaqueExternalPayload` validation before becoming `ContextCandidate`.

## Rejected path

Direct string injection in runtime loops without `ContextFacade` remains **out of scope** for new features (legacy engines documented under call-site inventory).
