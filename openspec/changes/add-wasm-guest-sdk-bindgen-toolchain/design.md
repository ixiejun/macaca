## Context

The SDK currently builds a static scaffold. Industrial developer workflow
requires WIT-driven generated bindings and local certification feedback.

## Goals / Non-Goals

- Goals: WIT input validation, generated binding plan, Rust guest scaffold,
  mock host import tests, package fixture generation, local certification
  report, sanitized diagnostics.
- Non-Goals: engine-specific generated code, application-specialized scaffold
  behavior, hardcoded workflows, SDK-owned provider construction, or SDK-owned
  runtime execution.

## Decisions

- Use Builder for scaffold generation.
- Use Adapter for bindgen backend so future languages can be added.
- Use existing runtime guest harness fixture semantics for local host import
  behavior without depending on a concrete provider.
- Keep SDK outputs as provider-neutral descriptors, source snippets, fixtures,
  and diagnostics.

## Governance

SDK remains a facade/developer API boundary. It can generate descriptors and
test fixtures, but it must not become a runtime provider, service composition
root, Web/CLI semantic owner, or engine-specific adapter.

## Risks / Trade-offs

- Full code generation can become large. Mitigation: start with deterministic
  generated source DTOs and fixture tests, then add CLI integration in a later
  shell-facing proposal.
- Generated code can drift from runtime contracts. Mitigation: reuse WIT labels,
  admission descriptors, and certification fixture checks.

## Migration Plan

Existing scaffold API remains. The new bindgen builder extends it with
WIT-driven generation.
