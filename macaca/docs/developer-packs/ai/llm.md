# AI LLM Pack

`pack.ai.llm.v1` provides provider-neutral chat, completion, model-routing
metadata, token estimation, budget inspection, and generation cancellation.
The pack is descriptor-only until a serviceized LLM provider is registered by
the runtime composition root.

Applications declare the pack; they do not hold provider credentials, choose
concrete model names in OS-layer code, or call model providers directly.

## Manifest Declaration

Required declarations block readiness when the pack is unavailable. Optional
declarations degrade with `ai_llm_provider_not_installed` and keep the
effective capability memento traceable.

```toml
[service_contract]
optional_packs = ["pack.ai.llm.v1"]
```

## Permission Scopes

- `ai.llm.invoke`: chat, completion, estimation, and cancellation intent.
- `ai.llm.route`: provider-neutral routing metadata and capability selection.
- `ai.llm.budget`: budget envelope inspection and usage preflight.

## Commands

- `llm.chat`: submits a bounded `LlmInvocation` for conversational generation.
- `llm.complete`: submits a bounded completion invocation.
- `llm.route_model`: asks the service to select a provider-neutral capability
  class without exposing concrete model names.
- `llm.estimate_tokens`: estimates usage from message and content references.
- `llm.inspect_budget`: returns the effective `LlmBudgetEnvelope`.
- `llm.cancel_generation`: requests cancellation for an in-flight generation.

## DTOs And Results

Core DTOs include `LlmInvocation`, `LlmMessage`, `LlmContentBlock`,
`LlmToolCall`, `LlmStreamFrame`, `LlmGeneration`, and `LlmBudgetEnvelope`.
Result envelopes use typed statuses for success, partial, denied, unavailable,
unsupported, conflict, quota exceeded, budget exceeded, cancellation, schema
mismatch, tool-policy-required, and provider failure.

## Examples

Minimal app-facing declaration:

```toml
[service_contract]
optional_packs = ["pack.ai.llm.v1"]
```

Provider-unavailable diagnostic:

```json
{
  "pack_id": "pack.ai.llm.v1",
  "required": false,
  "reason_code": "optional_pack_unresolved",
  "message": "ai_llm_provider_not_installed"
}
```

Canonical command payloads should reference private inputs:

```json
{
  "subject_ref": "session:current",
  "parameters": {
    "invocation_ref": "llm-invocation-ref",
    "message_ref": "message-ref",
    "budget_ref": "budget-ref"
  },
  "idempotency_key": "request-stable-key"
}
```

## Trace And Audit

Trace and audit events may include pack id, service id, command name, trace id,
budget counters, capability hash, status, and bounded error codes. They must not
include raw prompts, generated output, tool arguments, credentials, concrete
model names, raw provider payloads, or unbounded diagnostics.

## Provider Replacement

Provider classes are descriptor labels such as `hosted-model`, `local-runtime`,
`plugin`, `mock`, and `unavailable`. Runtime-host and plugin composition roots
bind concrete adapters after policy, entitlement, resource, metering, approval,
trace, and redaction decorators are installed.
