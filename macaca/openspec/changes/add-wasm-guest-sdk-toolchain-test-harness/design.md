## Context
Previous WASM phases added provider contracts, package admission, a default in-process runtime provider, resource governance, host import portal routing, and lifecycle checkpoint support. This phase adds a runtime-owned local harness so guest SDK/toolchain fixtures can be tested against the same provider-neutral DTOs as real runtime dispatch.

## Goals / Non-Goals
- Goals: provide runtime test doubles for host imports, deterministic fixture generation, WIT label drift checks, sanitized traces, and contract tests for common guest SDK proxy operations.
- Non-Goals: implement a full Rust guest crate, generate real language bindings, execute raw guest WASM memory, add IDE tooling, or publish to Store.

## Decisions
- Decision: Use Facade and Proxy in the harness API.
  Rationale: test authors can call service/storage/render proxies while the harness converts those calls into provider-neutral `ApplicationHostCommand` values.
- Decision: Use Builder for deterministic fixtures.
  Rationale: manifests, artifact descriptors, permissions, and host import expectations can be generated repeatably without hard-coded application-specific behavior.
- Decision: Use Adapter for WIT labels.
  Rationale: the runtime harness can validate WIT canonical labels against `ApplicationImport`/`ApplicationExport` without depending on a real binding generator.
- Decision: Use Test Double for local mock host imports.
  Rationale: success, denied, unavailable, and unsupported outcomes can be tested without launching a Macaca runtime, while using the same sanitized DTO/error vocabulary as the real host import portal.

## Risks / Trade-offs
- Mock drift from real host import bridge -> reuse `ApplicationHostCommand`, `ApplicationHostCommandResult`, and WASM host import metadata constants in tests.
- Over-building SDK behavior inside runtime -> keep this slice harness-only and provider-neutral.
- Sensitive data leakage in fixtures -> sanitize labels, traces, metadata, and outputs; never store raw WASM bytes, raw payloads, prompts, secrets, API keys, or environment values.
