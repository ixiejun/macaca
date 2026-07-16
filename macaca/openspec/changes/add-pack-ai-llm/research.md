# AI LLM Pack Research

## Purpose

This note records borrowed platform patterns, Macaca mapping, existing platform
inventory, and GitNexus memo evidence for `pack.ai.llm.v1`. The pack must
expose chat, completion, model routing, token estimation, budget inspection, and
cancellation through provider-neutral service commands. It must not leak model
provider APIs, prompts, credentials, raw provider payloads, or application
workflow logic into kernel, SDK, shell, or generic application framework code.

## Source Baseline

- OpenAI API documentation for model responses, structured outputs, tool calls,
  streaming, token usage, and embeddings/speech/vision adjacent patterns:
  <https://platform.openai.com/docs>
- Anthropic Messages API for message arrays, content blocks, streaming, tool use,
  and usage metadata: <https://docs.anthropic.com/en/api/messages>
- AWS Bedrock Converse API for provider-neutral conversation, streaming, tool
  use, guardrail, inference configuration, and model invocation:
  <https://docs.aws.amazon.com/bedrock/latest/userguide/conversation-inference.html>
- Google Vertex AI generative model APIs for model invocation, safety settings,
  function calling, streaming, and usage metadata:
  <https://cloud.google.com/vertex-ai/generative-ai/docs>
- Azure AI model inference and content safety documentation:
  <https://learn.microsoft.com/en-us/azure/ai-foundry/model-inference/>

## Borrowed Platform Patterns

- Provider APIs converge on structured message arrays, content blocks, tool-call
  metadata, streaming frames, usage/cost accounting, and explicit cancellation or
  timeout behavior. Macaca should model these as `LlmInvocation`,
  `LlmMessage`, `LlmContentBlock`, `LlmToolCall`, `LlmStreamFrame`,
  `LlmGeneration`, and `LlmBudgetEnvelope`.
- Mature APIs separate input messages, generation options, tool definitions,
  structured output constraints, and safety/guardrail policy. Macaca should
  keep policy and approval as service decorators rather than embedding provider
  guardrail names in SDK contracts.
- Streaming providers emit deltas, final usage, and terminal finish reasons.
  Macaca should require ordered frame sequence, finalization, late-frame
  handling, replay pointers, and cancellation evidence.
- Provider usage and budget data are necessary for admission, quota, rate, and
  cost control. Macaca should expose bounded usage counters and budget envelopes,
  never raw prompts or provider payloads, in trace/audit surfaces.
- Tool-call generation is not tool execution. Macaca must route generated tool
  calls back through owning capability services and policy gates.

## Macaca Mapping

- Descriptor: `pack.ai.llm.v1`, command namespace `llm.*`, scopes
  `ai.llm.invoke`, `ai.llm.route`, and `ai.llm.budget`.
- Commands: `llm.chat`, `llm.complete`, `llm.route_model`,
  `llm.estimate_tokens`, `llm.inspect_budget`, and `llm.cancel_generation`.
- Policy: validate declared pack scope, model/provider neutrality, input
  sensitivity, output retention, budget, rate, tool-call metadata, structured
  output schema, streaming resource bounds, and approval requirements before
  provider invocation.
- Trace/audit: emit declaration, admission, policy, entitlement, resource,
  service-call, streaming lifecycle, cancellation, health, snapshot,
  unavailable, and sanitized failure events.
- Redaction: record hashes, lengths, role counts, tool-call ids, schema ids,
  usage counters, latency, and bounded error codes instead of raw prompts,
  generated text, credentials, or provider responses.

## Existing Macaca Platform Inventory

- Existing `macaca_llm::LlmProvider`, `LlmMessage`, `LlmOptions`, and
  `LlmResponse` usage proves there is a legacy/provider abstraction that can
  inform, but not complete, the industrial pack contract.
- `LlmSystemServiceProvider`, `LlmRouter`, model selection profiles, and focused
  runtime surfaces provide serviceization material that must be wrapped behind
  `pack.ai.llm.v1` descriptors and typed commands before this child proposal is
  complete.
- `macaca-sdk::SystemFacade` already exposes `SystemLlmClient` and
  `UnavailableSystemLlmClient`, showing the Facade/Null Object pattern required
  for SDK discovery and unavailable diagnostics.
- `macaca-kernel::service_call` enforces trace-required dispatch and is the only
  acceptable execution path for future LLM pack commands.
- Runtime context-window and agentic-loop tests contain token-estimation and
  tool-call behavior that can provide compatibility lessons, but they are not
  provider-neutral pack DTOs or gates.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
