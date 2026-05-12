## Context

After the Application Platform contracts, SDK, YAML adapter, metadata service, and WASM skeleton exist, certification fixtures validate the full design intent: application ecosystem support must be generic, traceable, auditable, safe, and not demo-specific.

## Goals

- Add generic fixtures for major application forms.
- Add CertificationKit with Visitor-style traversal over manifest/ability/dependency declarations.
- Validate fail-closed/unavailable behavior for missing capabilities.
- Validate redaction and trace requirements.

## Non-Goals

- Do not run real network, Store, Payment, Web3/EVM, Plugin, or WASM execution.
- Do not certify business-specific apps.
- Do not delete deprecated compatibility paths.

## Decisions

- Decision: Use Visitor for certification traversal.
  Rationale: certification needs to inspect manifests, abilities, permissions, services, plugins, commerce, UI, and ABI consistently.

- Decision: Use Specification for each certification rule.
  Rationale: each rule should be independently testable and reusable by future Store submission checks.

- Decision: Use fixtures as data-only contracts.
  Rationale: certification should validate platform shape without external services.

- Decision: Use Memento-style certification reports.
  Rationale: reports must be serializable, auditable, and useful for future developer tooling.

## Risks / Trade-offs

- Risk: Fixtures become fake demos disconnected from production contracts.
  Mitigation: build fixtures through SDK builders and validate through the same app/proto contracts used by runtime code.

- Risk: Certification duplicates TestKit.
  Mitigation: SDK TestKit validates developer-side contract construction; CertificationKit validates platform-wide package readiness and safety.

- Risk: Tests leak raw data.
  Mitigation: include explicit redaction assertions.

## Migration Plan

1. Add certification module and report types.
2. Add generic fixtures through SDK builders.
3. Add integration contract tests.
4. Add redaction/fail-closed/unavailable assertions.
5. Run workspace validation.

## Trace / Audit

Certification reports must include fixture id, app id, ability ids, service ids, capability ids, operation, status, reason code, and trace id when supplied. They must not include prompt bodies, raw full manifest bodies, raw agent configs, raw WASM bytes, secrets, env, API keys, private keys, raw signatures, raw host payloads, or unbounded user input.
