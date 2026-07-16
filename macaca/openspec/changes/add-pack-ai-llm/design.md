# AI LLM Pack Design

## Context

`pack.ai.llm.v1` is a child proposal of the developer-pack industrial capability catalog. It makes chat, completion, routing, policy, budget, tool-call metadata, and model invocation diagnostics available through Macaca's microkernel-compatible service model. The pack must behave like an operating-system capability: declarative in application manifests, policy checked before use, provider-neutral at the SDK boundary, observable through trace/audit evidence, and replaceable by built-in, plugin, remote, mock, or unavailable providers.

## Research Synthesis

Mature platforms converge on the same pattern:

- Platform privacy models: raw user data must not enter logs or diagnostics when processed by intelligent services.
- Android runtime permission pattern: AI operations that touch private data inherit source permissions.
- Windows capability declaration pattern: model-backed operations must be visible in app capability metadata.
- Apple entitlement/privacy pattern: sensitive processing requires policy and developer-declared purpose.

Macaca should adapt those ideas as pack descriptors, service commands, policy decorators, and replayable audit records. The design does not copy platform API names; it preserves Macaca's own microkernel boundary and service runtime execution path.

## Goals

- Provide chat, completion, routing, policy, budget, tool-call metadata, and model invocation diagnostics.
- Expose stable pack id `pack.ai.llm.v1`, command namespace `llm.*`, permission scopes, SDK metadata, health, snapshot, and unavailable diagnostics.
- Route every operation through `SystemFacade` or focused SDK clients into the canonical service runtime path.
- Return structured `unavailable`, `unsupported`, `denied`, `conflict`, `quota_exceeded`, and `failure` results.
- Emit sanitized trace/audit events for declaration, admission, policy, entitlement/resource decisions, service calls, provider health, snapshots, and replay.

## Non-Goals

- Do not implement an application-specific feature, workflow, UI, or business rule.
- Do not put concrete provider construction in the microkernel, SDK, shells, or generic application framework.
- Do not expose raw secrets, prompts, manifests, package bytes, credentials, raw provider payloads, private keys, signatures, or unbounded output in logs, traces, snapshots, SDK diagnostics, or examples.
- Do not silently degrade to a different provider, fake success, or bypass policy when the declared provider is absent.

## Ownership And Boundaries

- Pack id: `pack.ai.llm.v1`.
- Family: `ai`.
- Backing service owner: LLM service provider.
- SDK surface: `sdk.packs.ai.llm`.
- Command namespace: `llm.*`.
- Microkernel ownership: identity, service-call evidence, policy facade, trace/audit primitives only.
- Application framework ownership: manifest declaration, app-scoped permission declarations, lifecycle/effective-capability projection.
- Runtime-host ownership: provider registration and decorators only through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `llm.chat` | Typed command/result DTO for chat | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `llm.complete` | Typed command/result DTO for complete | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `llm.route_model` | Typed command/result DTO for route model | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `llm.estimate_tokens` | Typed command/result DTO for estimate tokens | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `llm.inspect_budget` | Typed command/result DTO for inspect budget | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `llm.cancel_generation` | Typed command/result DTO for cancel generation | Requires trace, policy decision, structured result, and sanitized audit evidence |

Every command must define a typed command DTO, typed success result, typed partial-result shape when streaming or asynchronous, typed error result, redaction policy, idempotency semantics where side effects exist, and replay metadata.

## Permission, Policy, Resource, Entitlement, And Approval Model

Initial permission scopes:

- `ai.llm.invoke`
- `ai.llm.route`
- `ai.llm.budget`

Policy defaults:

- Scope every command to application id, tenant id, session id, task id, and trace id when available.
- Apply model/provider neutrality, budget/rate policy, prompt/output redaction, evaluation trace, and no model-name routing in OS code.
- Require explicit approval for commands that cross user-sensitive, financial, identity, device, host, network, external-recipient, or irreversible side-effect boundaries.
- Enforce resource budgets for time, memory, storage, network, provider quota, streaming output, and retained snapshots.
- Return `denied` for policy rejection, `unavailable` for absent providers or entitlements, `unsupported` for unknown commands, and `quota_exceeded` for bounded-resource rejection.

## SDK Discovery And Examples

SDK discovery must return pack metadata, lifecycle, service mappings, command schemas, permission scopes, policy templates, examples, availability, health, provider class, version compatibility, and sanitized diagnostics.

- SDK helper example: `sdk.packs.ai.llm.chat(command)` builds a canonical traced service call; it never constructs providers.
- SDK helper example: `sdk.packs.ai.llm.complete(command)` builds a canonical traced service call; it never constructs providers.
- SDK helper example: `sdk.packs.ai.llm.route_model(command)` builds a canonical traced service call; it never constructs providers.

Examples must use generic handles and synthetic data. They must not bake in application names, provider names, credentials, business workflows, or domain-specific routing.

## Trace, Audit, Health, Snapshot, And Replay

Required event names:

- `llm_pack_declared`
- `llm_pack_admission_validated`
- `llm_pack_policy_decision`
- `llm_pack_service_call_requested`
- `llm_pack_service_call_succeeded`
- `llm_pack_service_call_failed`
- `llm_pack_unavailable`
- `llm_pack_snapshot_recorded`

Events include pack id, service id, command name, trace id, application/session/task/tenant identifiers when available, policy decision, provider class, latency, bounded resource counters, stable capability hash, and bounded error code. Snapshots include descriptor version, provider health, command availability, policy template hash, and sanitized replay pointers.

## Implementation Slices

1. Descriptor and contract slice: pack descriptor, command schemas, permissions, policy template, health/snapshot DTOs, unavailable diagnostics.
2. Admission and resolver slice: required/optional declaration handling, lifecycle checks, service mapping checks, permission validation, effective capability memento.
3. Service slice: LLM service provider command handlers or unavailable provider, lifecycle, health, snapshot, shutdown, and structured error behavior.
4. SDK slice: discovery APIs, typed command helper builders, examples, diagnostics, and Null Object behavior.
5. Observability slice: trace/audit events, replay tests, snapshot sanitization, and metrics.
6. Gates slice: OpenSpec validation, DTO compatibility, dependency-boundary tests, no-direct-provider-call tests, canonical execution-path tests, file-size gates.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders; it does not construct providers.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider selection, unavailable behavior, policy routing, and version compatibility are replaceable.
- **Decorator**: trace, policy, resource, entitlement, approval, metering, and redaction wrap every call.
- **Specification**: admission validates pack id, lifecycle, commands, permissions, policy, and service mapping.
- **Observer**: trace, audit, health, and service events are subscribable and replayable.
- **Memento**: effective capability reports and snapshots preserve bounded recovery state.
- **Abstract Factory**: optional providers register only through approved composition roots.

## Risks And Mitigations

- Risk: broad capability becomes an OS-layer business workflow. Mitigation: keep the pack contract generic and place domain/provider semantics in replaceable services.
- Risk: SDK helpers become a second execution path. Mitigation: helpers only build canonical service-call commands and are covered by no-direct-provider-call gates.
- Risk: preview or unavailable providers look callable. Mitigation: availability validators require descriptor, service registration, command schema, permission, entitlement, and health evidence before callable state.
- Risk: observability leaks sensitive data. Mitigation: event schema permits identifiers, hashes, counters, bounded codes, and sanitized snippets only.

## Supplier Capability Matrix

| Supplier pattern | Macaca contract element |
| --- | --- |
| Messages/content blocks | `LlmMessage`, `LlmContentBlock`, redaction profile |
| Tool calls/results | `LlmToolCall`, `LlmToolResultRef`, declared tool capability binding |
| Streaming deltas | `LlmStreamFrame`, sequence number, finish reason |
| Structured output | schema reference, validation mode, mismatch diagnostics |
| Usage and budget | input/output token counters, cost estimate, quota result |

## Domain Model

- `LlmInvocation`: canonical request containing messages, optional multimodal
  references, generation options, tool configuration, structured-output schema,
  idempotency key, and redaction profile.
- `LlmGeneration`: final or partial generation with content blocks, finish
  reason, safety/policy summary, usage counters, and provider-neutral metadata.
- `LlmStreamFrame`: ordered frame for streamed content, tool-call deltas, usage
  updates, errors, cancellation, and finalization.
- `LlmBudgetEnvelope`: preflight and postflight budget state with token, cost,
  rate, and retained-output limits.
- `LlmCapabilityDescriptor`: provider-neutral feature flags such as streaming,
  tool calls, structured output, multimodal inputs, context-window bands, and
  redaction support.

## Execution State Machine

```text
created -> admitted -> budget_reserved -> invoking -> streaming -> completed
created -> admitted -> denied
invoking|streaming -> cancelling -> cancelled
invoking|streaming -> failed
completed -> replayable_summary_recorded
```

Streaming frames must be ordered and replayable by sequence number. Cancellation
must return a structured terminal result even when the provider reports a late
final frame.

## Additional Industrial Commands

- `llm.validate_structured_output`: validate a generation against a declared
  schema without invoking a provider.
- `llm.inspect_capabilities`: return provider-neutral features and context
  window bands without leaking provider secrets.
- `llm.stream_chat`: explicit streaming variant when the SDK needs a framed
  stream contract rather than a single result.

## LLM-Specific Risks

- Risk: provider/model routing leaks into OS code. Mitigation: descriptors
  expose capability classes and policy tags; concrete model names remain
  provider data behind service adapters.
- Risk: tool-call output bypasses permissions. Mitigation: generated tool calls
  are metadata only until a declared tool/capability service validates and
  executes them through its own policy gate.
- Risk: prompts leak through replay. Mitigation: replay stores message hashes,
  token counters, redaction profile, and bounded summaries only.
