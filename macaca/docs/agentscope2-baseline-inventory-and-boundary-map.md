# AgentScope Java 2.0 Baseline Inventory And Macaca Boundary Map

## Purpose

This document is the baseline audit for OpenSpec change
`upgrade-framework-to-agentscope2`. It records the current AgentScope Java 2.0
source and documentation surface, the current Macaca framework surface, the
Macaca ownership boundary for each capability, and the known direct consumers
that must be migrated without breaking existing behavior.

The inventory is intentionally provider-neutral. Macaca should gain AgentScope 2.0
parity through stable framework/service contracts, not through application
workflow branching or provider-name routing.

## Evidence Sources

- Upstream docs URL checked on 2026-06-08:
  `https://java.agentscope.io/v2/zh/docs/index.html`.
- The online docs page advertises Sphinx-generated AgentScope Java docs with
  `article:modified_time` of `2026-06-02T16:19:40+00:00`.
- Local upstream source repository:
  `/Users/quantum/Code/dev/agentscope-java`.
- Local Macaca repository:
  `/Users/quantum/Code/dev/agent/macaca`.

## AgentScope Java 2.0 Source Inventory

Top-level source count from local repository:

| Area | Main Java files | Notes |
| --- | ---: | --- |
| `agentscope-core` | 410 | Core agent, event, message, middleware, model, tool, state, session, permission, skill, RAG, shutdown, formatter, tracing APIs. |
| `agentscope-harness` | 225 | Harness agent, workspace, filesystem, sandbox, memory/compaction, skills, subagents, task repository, plan mode, tools. |
| Examples | 547 | Builder, Claw, CodingAgent, DataAgent, and documentation examples. Used for parity behavior checks, not OS ownership. |
| Extensions and starters | 362 | Protocol, AG-UI, A2A, Chat Completions web, scheduler, Nacos, RocketMQ, Higress, RAG, memory, session, skill repositories, Studio, training, Spring Boot starters. |
| Total local main Java files | 1544 | Count includes examples and optional integrations. |

### Core Package Inventory

| Core package | Files | Macaca upgrade concern |
| --- | ---: | --- |
| `formatter` | 86 | Provider message conversion, response parsing, media handling. Macaca keeps provider adapters separate from consumer ABI. |
| `tool` | 57 | Tool base, registry, groups, MCP wrappers, context injection, suspend semantics. Must cross Macaca tool/MCP services before side effects. |
| `model` | 54 | Model cards, registry, execution config, transport, retry/fallback, structured output. Must be owned by LLM service/runtime-host adapters. |
| `event` | 29 | Canonical typed event stream. Macaca maps to `AgentEvent` and protocol adapters. |
| `agent` | 26 | Agent base contracts, streamable agent, runtime context, user input, subagent event bus. |
| `message` | 23 | Role-validated messages, content blocks, tool call/result state, usage/generate reason metadata. |
| `hook` | 21 | Legacy hook events/dispatcher. Macaca bridges through middleware and keeps direct hook paths deprecated. |
| `skill` | 17 | Dynamic skills and repositories. Macaca delegates ownership to skill/plugin services. |
| `credential` | 11 | Provider credentials/cards. Macaca must not store concrete credentials in framework. |
| `plan` | 11 | Plan notebook and hint middleware. Macaca keeps TaskBoard ownership in task/execution-control services. |
| `state` | 10 | Agent state, session key, tool/task/read-cache state. Macaca maps to provider-neutral `AgentState`. |
| `shutdown` | 10 | Graceful shutdown, active requests, partial reasoning policy. Needed for continuous 24/7 operation. |
| `permission` | 8 | Permission engine, rules, decision modes. Macaca policy/service decorators must gate side effects. |
| `middleware` | 8 | Agent/reasoning/acting/model/system-prompt middleware. |
| `memory` | 8 | In-memory/state-backed/LTM hooks/tools. Macaca delegates memory/context to services. |
| `rag` | 7 | RAG hooks/tools. Macaca delegates retrieval to memory/context/retrieval services. |
| `session` | 6 | In-memory/json session. Macaca bridges to persist/session providers. |
| `tracing`, `interruption`, `workspace`, `exception`, `util` | 15 | Trace export, interruption, utility, exception, workspace support. |

### Harness Package Inventory

| Harness package | Files | Macaca boundary |
| --- | ---: | --- |
| `sandbox` | 89 | Driver/filesystem/sandbox services; no silent host fallback when sandbox is required. |
| `skill` | 30 | Skill/plugin services; framework only adapts prompts and tool declarations. |
| `filesystem` | 27 | Filesystem/driver/sandbox service adapters. |
| `subagent` | 17 | Task/execution-control service and framework event source identifiers. |
| `middleware` | 15 | Harness middleware over workspace, compaction, skills, subagents, plan mode, sandbox lifecycle. |
| `tool` | 12 | Filesystem, memory, shell, task, plan, skill, agent tools through service boundaries. |
| `store` | 11 | Store/persist service ownership; no framework-owned backend policy. |
| `memory` | 9 | Context/memory services plus framework middleware. |
| `workspace` | 6 | Workspace metadata and path policy through application/filesystem boundaries. |
| `tools`, `session` | 6 | MCP tool config and workspace session support. |

## AgentScope Java 2.0 Documentation Inventory

Local v2 Chinese documentation files:

| Docs group | Files |
| --- | --- |
| Quick start and overview | `docs/v2/zh/docs/index.md`, `quickstart.md`, `change-log.md`, `others/faq.md`, `others/going-to-production.md` |
| Core building blocks | `agent.md`, `message-and-event.md`, `middleware.md`, `model.md`, `permission-system.md`, `tool.md` |
| Harness | `architecture.md`, `workspace.md`, `context.md`, `memory.md`, `filesystem.md`, `sandbox.md`, `skill.md`, `subagent.md`, `plan-mode.md` |
| Protocol integrations | A2A, Agent Protocol, AG-UI |
| Ecosystem integrations | Chat Completions web, Kotlin, Studio, training |
| Infrastructure integrations | Scheduler, RocketMQ, Nacos, Higress |
| Memory/RAG/session/skill integrations | Mem0, ReMe, Bailian memory/RAG, Dify, Haystack, RAGFlow, simple RAG, Redis/MySQL session, Git/MySQL skill repositories |

## Current Macaca Framework Inventory

| Macaca area | Files | Status relative to AgentScope 2.0 |
| --- | --- | --- |
| Provider contract | `provider_contract.rs`, `provider_contract_tests.rs`, runtime-host `framework_provider.rs` | Provider-neutral descriptor, health, snapshot, unavailable provider, command/result/stream DTOs, runtime-host composition root exist. |
| Message | `message.rs`, `message_tests.rs` | Role validation, `DataBlock`, `HintBlock` exist; stable ids on every block, tool states, usage/generate reason fields are still incomplete. |
| Event | `event_contract.rs`, `event_contract_projection.rs`, `event_contract_tests.rs` | Typed lifecycle/model/content/tool/HITL/external events and final-message projection exist. |
| Runtime context/state | `runtime_context.rs`, `runtime_context_tests.rs` | RuntimeContext, SessionKey, AgentState, in-memory AgentSessionStore exist. Legacy session/state bridge is pending. |
| Middleware | `middleware.rs` | Five-stage chain and legacy hook dispatcher exist. |
| ReAct | `react_agent.rs`, `react_agent_steps.rs`, `react_agent_tests.rs`, legacy `react_agent.rs` | Additive AgentScope 2.0-style event-loop exists; retry/fallback, resume, graceful shutdown, and full consumer migration are pending. |
| Model/formatter | `model.rs`, `formatter.rs`, `adapter_llm.rs`, `llm_wire.rs` | Legacy formatter/model traits exist; provider-neutral model registry/credential/card contracts remain delegated to LLM service. |
| Tools/MCP | `tool.rs`, `mcp.rs` | Legacy toolkit and MCP client exist; AgentScope 2.0 tool suspend/context/group/meta/schema-only parity is pending. |
| Memory/session/state/plan | `memory.rs`, `session.rs`, `state.rs`, `plan.rs` | Compatibility modules exist; service-backed bridges are pending. |
| Protocol | `a2a.rs` | A2A compatibility exists; AgentScope 2.0 event/task/protocol adapters are pending. |
| Runtime-host glue | `framework_runtime_agent_service.rs`, `framework_provider.rs` | Runtime-host construction is now the approved provider composition root. Shell construction adapters remain migration debt. |

## Capability To Macaca Boundary Map

| AgentScope 2.0 capability | Macaca owner boundary | Design pattern | Migration rule |
| --- | --- | --- | --- |
| Agent runtime provider | `macaca-framework` contract + `macaca-runtime-host` composition root | Facade, Abstract Factory, Strategy, Null Object | Consumers call provider-neutral commands/events; only runtime-host constructs providers. |
| Message/content blocks | `macaca-framework` contract | Command DTO, Specification | Preserve role validation and structured blocks; no provider-specific public types. |
| Event stream | `macaca-framework` contract, shell/protocol adapters | Observer, Memento | `stream_events` is canonical; compatibility reply/call paths must project from events. |
| RuntimeContext | Framework contract carried by service/runtime command envelopes | Command, Context Object | Per-call metadata must not be persisted as durable state. |
| AgentState/session | Framework contract + persist/memory/context service bridges | Memento, Bridge | Durable state isolated by tenant/application/session/agent key. |
| Middleware | Framework provider | Chain of Responsibility, Decorator | Service decorators still wrap side effects at service/runtime boundary. |
| Model registry, credentials, retry/fallback | LLM service/runtime-host adapter | Strategy, Adapter | Framework must not hardcode model/provider names or credentials. |
| Tools and permissions | Tool service + framework adapter + policy service | Command, Strategy, Decorator | Policy before side effects; unsupported/denied/unavailable are structured. |
| MCP | MCP/tool service + framework adapter | Adapter, Strategy | Stdio/SSE/HTTP/streamable HTTP parity must be service-backed. |
| Harness workspace | Framework harness + application/filesystem/context services | Facade, Adapter | Workspace context can feed middleware; path policy remains service-owned. |
| Harness memory/context | Context/memory services + harness middleware | Decorator, Memento | Compaction, eviction, memory flush must not bypass context/memory services. |
| Harness filesystem/sandbox | Driver/filesystem/sandbox services | Adapter, Null Object | Required sandbox absence returns unavailable/denied; never silent host execution. |
| Harness skills | Skill/plugin services + harness adapter | Repository, Adapter | Skill discovery, curation, loading, promotion remain service/plugin governed. |
| Harness subagents/tasks | Task/execution-control services + framework event forwarding | Command, Observer | Subagent background/sync work must use task/execution-control boundaries. |
| Plan mode | Framework agent-local state + task service boundary | State, Memento | Plan mode cannot own or mutate TaskBoard except through approved service commands. |
| Protocol adapters | Gateway/protocol services and shell adapters | Adapter | A2A, AG-UI, Chat Completions, Agent Protocol consume `AgentEvent`. |
| RAG/memory/session/skill repositories | Optional services/providers | Adapter, Null Object | Optional extension absence is explicit unavailable/unsupported/denied. |

## Direct Macaca Consumers And Migration Risk

Direct dependency crates:

| Consumer | Current dependency | Risk | Migration direction |
| --- | --- | --- | --- |
| `macaca-runtime-host` | Depends on `macaca-framework` with `macaca-compat`; imports `model::ToolChoice`, MCP primitives, toolkit, and new provider contract. | Medium | Keep runtime-host as composition root; migrate MCP/tool/model calls into service-backed adapters. |
| `macaca-web` | Depends on `macaca-framework` with `macaca-compat` and `service-clients`; constructs legacy `ReActAgent`, `Toolkit`, hooks, session modules, and SSE adapters. | High | Migrate Web to stable framework commands/events or focused SDK clients; leave deprecated paths until production callers are gone. |
| `macaca-context` | Uses framework chat options in context engine paths. | Medium | Replace direct framework model DTO usage with provider-neutral context/LLM service DTOs when approved. |
| Integration tests/gates | Reference framework file sizes, route boundaries, MCP, A2A, payment and runtime host paths. | Medium | Update gates after each slice; do not loosen constitutional checks except with explicit migration allowlist. |

High-risk direct files and why they matter:

| File/group | Current use | Migration risk |
| --- | --- | --- |
| `crates/shells/macaca-web/src/framework_runner/*` | Builds legacy `ReActAgent`, `HookedAgent`, `Toolkit`, `OpenAiFormatter`, service model adapter, hooks, SSE emitters. | Highest risk consumer. Must move to runtime-host provider/factory or typed `AgentEvent` adapters without breaking `/api/chat/v2`. |
| `crates/shells/macaca-web/src/framework_agent_construction_shell_adapter.rs` | Shell construction adapter for legacy runtime-host port. | Temporary migration seam; should shrink as runtime-host provider construction matures. |
| `crates/shells/macaca-web/src/framework_toolkit/*` | Builds tools/toolkit and policy around framework toolkit. | Tool service bridge needed before AgentScope 2.0 tool parity. |
| `crates/shells/macaca-web/src/chat_orchestrator/*` | Routes chat v2, framework path, stop route, session persistence. | Must render from typed `AgentEvent` while preserving response shape. |
| `crates/runtime/macaca-runtime-host/src/mcp_runtime.rs` | Owns service-side MCP runtime but still imports framework MCP/tool primitives. | Acceptable migration seam; future MCP DTOs should move to provider-neutral service contracts. |
| `crates/runtime/macaca-runtime-host/src/framework_runtime_agent_service.rs` | Existing service-backed port over shell construction. | Compatibility path until `AgentRuntimeProvider.stream_events` covers Web execution. |

## Boundary Invariants For Implementation Slices

- Kernel must not import AgentScope 2.0 concrete provider, framework implementation,
  LLM provider, tool provider, driver, skill, MCP, gateway, payment, Web3, EVM,
  or shell code.
- `macaca-runtime-host` is the approved composition root for framework providers.
- Shells may adapt inputs and render events, but must not own agent execution
  semantics after migration.
- Framework code must not hardcode application names, workflow names, model
  names, driver names, provider names, chain names, payment names, or gateway
  names in control flow.
- All provider calls must require trace and return structured unavailable,
  unsupported, denied, or failed outcomes.
- Logs and snapshots must stay bounded and sanitized.
