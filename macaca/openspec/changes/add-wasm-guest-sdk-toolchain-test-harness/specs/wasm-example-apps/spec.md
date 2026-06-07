## ADDED Requirements
### Requirement: Runtime harness provides example app fixtures
Macaca SHALL provide deterministic runtime harness fixtures for headless, GenUI render, memory/context import, and service unavailable WASM application shapes.

#### Scenario: Example fixture generation
- **WHEN** a developer or contract test requests example fixtures
- **THEN** the harness SHALL return metadata-only fixtures with manifest-like identifiers, required imports, permissions, host command examples, and expected mock outcomes.

#### Scenario: Fixture safety
- **WHEN** example fixture metadata is logged or returned
- **THEN** it SHALL NOT include raw WASM bytes, raw payloads, prompts, secrets, API keys, private keys, environment values, or provider output.
