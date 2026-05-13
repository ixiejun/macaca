## Context
Previous WASM phases added provider-neutral runtime traits, metadata-only package admission, resource governance, host import routing, lifecycle checkpointing, and a runtime-owned guest/toolchain harness. This phase adds the certification/conformance layer that proves those contracts can be audited together without executing arbitrary guest code or leaking raw artifacts.

## Goals / Non-Goals
- Goals: define certification profiles, reusable conformance fixtures, negative security cases, sanitized report mementos, and a hardened provider envelope that can be tested with default/unavailable/mock providers.
- Non-Goals: implement a real sandbox daemon, compile third-party WASM components in this slice, add Store business review, or create language-specific SDKs.

## Decisions
- Decision: Use Specification for certification checks.
  Rationale: ABI, resource, import, lifecycle, observability, and security checks can be composed and audited without hard-coded application or provider names.
- Decision: Use Visitor over generated fixture bundles.
  Rationale: the harness can evaluate manifests, artifact descriptors, commands, provider descriptors, and negative cases uniformly.
- Decision: Use Memento for reports.
  Rationale: certification output must be immutable, serializable, bounded, and sanitized for logs, Store tooling, and CI artifacts.
- Decision: Use Template Method for certification profiles.
  Rationale: dev/default/hardened profiles share the same certification skeleton while selecting stricter rule sets.
- Decision: Use Adapter for the hardened provider mock.
  Rationale: out-of-process execution is a deployment profile, not a new application semantic, so the mock adapter must expose the same provider-neutral contract as default and unavailable providers.

## Risks / Trade-offs
- Risk: certification only validates happy paths. Mitigation: every profile must include negative security fixtures and fail-closed reason codes.
- Risk: hardened provider becomes a second runtime semantic. Mitigation: hardened envelope maps to existing provider-neutral DTOs and shares conformance tests.
- Risk: reports leak raw payloads or secrets. Mitigation: reports store only bounded identifiers, reason codes, profile labels, and sanitized diagnostics.
