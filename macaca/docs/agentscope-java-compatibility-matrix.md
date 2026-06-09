# AgentScope Java 2.0 Compatibility Matrix

## Purpose

This matrix tracks the Macaca Agent OS upgrade from the current AgentScope
1.x-style `macaca-framework` implementation to AgentScope Java 2.0 capability
parity.

The statuses below are intentionally provider-neutral. A row may be
`delegated-to-service` when Macaca's constitution assigns the behavior to a
system service rather than to `macaca-framework` itself.

## Status Values

- `missing`: no Macaca implementation yet.
- `partial`: Macaca has related behavior, but it is not AgentScope 2.0-equivalent.
- `equivalent`: Macaca has AgentScope 2.0-equivalent behavior.
- `delegated-to-service`: behavior belongs to a Macaca service/provider boundary.
- `compat-only`: retained only for migration compatibility.

## Completion Matrix

| Area | AgentScope Java 2.0 capability | Macaca status | Owner boundary | Notes |
| --- | --- | --- | --- | --- |
| Provider contract | Descriptor, health, snapshot, capability matrix, unavailable provider | equivalent | `macaca-framework` contract + runtime-host provider | Provider-neutral command/result/stream DTOs exist in `provider_contract.rs`; runtime-host owns AgentScope 2.0 provider construction through `framework_provider.rs` with injected agent factory strategy and unavailable fallback. |
| Message | Role-validated messages and typed content blocks | equivalent | `macaca-framework` contract | AgentScope 2.0 role validation helpers, `DataBlock`/`HintBlock`, stable block-id projection, tool call/result state projection, usage metadata, and generate-reason metadata are implemented. |
| Event stream | Typed `AgentEvent` lifecycle/model/content/tool/HITL events | equivalent | `macaca-framework` contract + shell adapters | Typed event DTOs, sequence-order validation, final-message accumulator, canonical `stream_events`, compatibility `call` projection, Web SSE projection, A2A, AG-UI, and Chat Completions adapters are implemented. |
| Runtime context | Per-call `RuntimeContext` | equivalent | `macaca-framework` contract + service command envelope | Provider-neutral per-call context exists, runtime-host creates it from trace-required framework commands, and durable state is kept separate from runtime extras. |
| Agent state/session | `AgentState + SessionKey + Session` | equivalent | framework contract + persist/memory/context services | Durable state/session contracts and compatibility bridges for existing `SessionMementoStore`/`RemovedStatePrimitive` are implemented. |
| Middleware | Five-stage middleware | equivalent | framework provider | Five-stage middleware contract, legacy hook dispatcher, and ReActAgent system-prompt/reasoning/model-call/acting wiring are implemented. |
| ReAct loop | AgentScope 2.0 ReAct loop with event stream, HITL, retry/fallback | equivalent | framework provider | `ReActAgent` emits typed events, reconstructs final messages, supports retry/fallback, HITL/external suspend/resume, structured output, interruption, max-iteration, and graceful shutdown paths. |
| Model registry | Provider id resolution, credentials, retry/fallback | delegated-to-service | LLM service/runtime-host provider | Framework should consume provider-neutral model service adapters. |
| Toolkit | AgentScope 2.0 tool base, groups, meta tools, context injection | equivalent | tool service + framework adapter | Provider-neutral `tool_contract` and the ReActAgent legacy toolkit bridge provide schema-only/meta/streaming/context injection/suspend semantics with guarded side-effect execution. |
| Permission/HITL | allow/ask/deny and resume events | equivalent | policy/execution-control services + framework middleware | `ToolPermissionEngineBridge` and ReActAgent enforce ask/deny/external suspend before side effects and support confirmation/external result resume. |
| MCP | stdio/SSE/HTTP/streamable HTTP, filters, elicitation | equivalent | MCP/tool services + framework adapter | Provider-neutral MCP descriptors/registry, stdio/HTTP/SSE/streamable HTTP metadata, filters, query/header config, protocol version, elicitation, and runtime-host descriptor synchronization are implemented. |
| Harness workspace | Workspace-driven persona/context/tools/skills/subagents | equivalent | framework harness + filesystem/context services | Harness workspace context is injected through middleware and service-bound adapters; filesystem/sandbox access remains policy-bound. |
| Harness context/memory | compaction, tool-result eviction, memory flush | equivalent | context/memory services + harness middleware | Context compaction, tool-result eviction, pre-truncation, overflow retry signaling, memory flush port, and memory tool descriptors are implemented. |
| Harness filesystem/sandbox | local/remote/sandbox specs, lease, snapshot | delegated-to-service | driver/filesystem/sandbox services | Harness exposes filesystem/sandbox ports and unavailable adapters; concrete execution remains delegated and host fallback is forbidden when unavailable. |
| Harness skills | repositories, dynamic skills, curation | delegated-to-service | skill/plugin services + harness adapter | Macaca skill governance remains owner. |
| Harness subagents | declarations, sync/background tasks, event forwarding | equivalent | framework harness + task/execution-control services | Harness subagent declarations, sync/background delegation envelopes, task repository port, unavailable adapters, and stable event source identifiers are implemented. |
| Harness plan mode | read-only planning, plan storage, HITL exit | equivalent | framework agent state + task service boundary | Plan mode uses agent-local state and preserves the TaskBoard service boundary. |
| Protocol adapters | A2A, AG-UI, Agent Protocol, Chat Completions | equivalent | gateway/protocol services + event adapters | A2A, AG-UI, and Chat Completions consume typed `AgentEvent`; Agent Protocol and extension families are represented as service-backed optional adapters. |
| RAG/memory extensions | RAG providers, Mem0/ReMe/Bailian | delegated-to-service | memory/context/retrieval services | AgentScope 2.0 marks some legacy RAG/memory APIs as moving targets. |
| Session extensions | MySQL/Redis sessions | delegated-to-service | persist/session provider boundary | Framework uses provider-neutral session contract. |
| License compliance | Apache-2.0 adapted-source notices | equivalent | repository compliance gate | AgentScope Java third-party notice and executable adapted-source header gate are implemented. |

## Compatibility Removal Rule

Update this matrix after each implementation slice. A compatibility path may be
removed only when all affected rows are `equivalent`, `delegated-to-service`, or
approved `compat-only`, and the relevant OpenSpec tasks and boundary gates pass.
