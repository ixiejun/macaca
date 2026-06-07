# Design: Application LLM Model Selection

## Context

Macaca already contains the main routing primitives: `service.llm` typed
commands, `SystemLlmClient`, and `LlmRouter::resolve_selection`. The missing
piece is a complete application-facing path that lets an app discover sanitized
provider/model options and submit a selected route without the Web shell or an
application becoming the owner of LLM routing semantics.

## Goals

- Let applications discover model choices from backend-owned `service.llm`
  capabilities.
- Let application execution carry request-level provider/model hints into
  route resolution.
- Keep LLM provider selection provider-neutral, traceable, auditable, and
  replaceable.
- Return structured unavailable or unsupported diagnostics for missing
  providers/models.

## Non-Goals

- Do not hardcode provider, model, app, workflow, or business-domain logic.
- Do not expose secrets, provider base URLs, raw prompts, raw provider payloads,
  or unbounded diagnostics.
- Do not make Web, frontend, or app-owned UI the semantic owner of routing.
- Do not introduce manifest-scoped model entitlement policy in this change;
  leave it as a follow-up hardening phase.

## Architecture

The design uses a service-first chain:

```text
Application UI
  -> generic app UI bridge
  -> Web shell adapter
  -> SystemLlmClient / service.call
  -> service.llm
  -> LlmRouter / provider strategy
```

`service.llm` owns catalog reads, provider capability reads, route resolution,
and actual chat dispatch. Web and frontend only adapt transport and render
sanitized state.

## Design Patterns

- Facade: `SystemLlmClient` remains the stable typed facade for shells and SDK
  users.
- Command: model catalog reads, provider capability reads, route resolution,
  and chat dispatch remain typed commands/results.
- Bridge/Adapter: app-owned UI bridge converts UI requests into service calls
  without provider-specific behavior.
- Strategy: `LlmRouter` owns provider/model selection and fallback order.
- Decorator: trace, policy, budget, audit, and metering metadata wrap the
  selection and execution boundary.
- Memento: sanitized catalog snapshots and selected route metadata are replayable
  without secrets or raw prompts.
- Specification: route validation rejects unavailable providers, unsupported
  models, or undeclared bridge capabilities through structured diagnostics.

## Key Decisions

### Service-owned catalog

`service.llm` SHALL expose all configured runtime provider rows that are safe to
show, including unavailable rows with sanitized reason codes. The catalog must
not pretend every configured model is usable; it reports known defaults and
known static model hints only when the provider or config supplies them.

### Request override precedence

Request-level model/provider hints SHALL have higher precedence than agent,
application, and system defaults, but the hint is still a request to the
service. `service.llm` remains responsible for accepting, rejecting, or
resolving fallback behavior.

### Thin shell adapter

Web MAY expose generic HTTP or bridge endpoints for model catalog and route
resolution, but those endpoints SHALL call the service facade and SHALL NOT own
provider/model interpretation.

### WASM session metadata

When a WASM application execution path does not directly invoke an LLM, the
requested route and resolved route still need to be stored as bounded session
metadata so replay can prove the user's intended model selection.

## Risks

- Existing provider metadata is currently single-profile shaped in runtime-host;
  implementation must extend it without making unavailable providers disappear.
- If only the UI selector is implemented, execution may still use the default
  provider/model silently.
- If route metadata includes raw prompts or provider responses, audit surfaces
  may leak sensitive data.
- If app-owned UI bypasses declared bridge capabilities, model selection will
  weaken the application permission model.

## Validation

- Validate OpenSpec with `openspec validate add-application-llm-model-selection --strict`.
- Add service tests for catalog rows, unavailable rows, and request override
  precedence.
- Add Web/API tests proving `/api/chat/v2` propagates the selected model route.
- Add bridge/UI checks proving the Codex WASM Workbench loads catalog data and
  submits selected route hints.
- Smoke test a real Codex WASM Workbench task and verify session/audit metadata
  contains sanitized route intent and effective route details.
