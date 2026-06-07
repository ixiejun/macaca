## Context

Macaca OS already exposes a generic WASM Component Model host-command chain and an `agent_delegate` bridge. The Codex Workbench application package can therefore express a collaboration workflow without adding OS-layer branches for Codex, task type, programming language, application name, or business domain.

## Goals

- Make all four declared Workbench agents participate in production execution.
- Let the coordinator LLM decide task complexity and collaboration depth from the actual request.
- Preserve traceable handoffs between agents by passing previous command outputs through host-command result placeholders.
- Keep application-specific prompts, role behavior, and UI expectations inside `apps/codex-wasm-workbench`.

## Non-Goals

- Do not add conditional routing or complexity heuristics to Macaca OS.
- Do not add Codex-specific logic to runtime-host, service runtime, SDK, Web shell, or frontend.
- Do not change the application execution protocol, event persistence, or replay contracts.

## Design Patterns

- **Command**: Each agent delegation remains a provider-neutral `agent_delegate` host command with typed payload, metadata, and trace.
- **Chain of Responsibility**: Coordinator, planner, coder, and reviewer form an application-owned handoff chain. Each role consumes prior outputs and contributes a bounded next-step artifact.
- **Strategy**: Complexity and collaboration depth are selected by the coordinator model from the task context, not by hardcoded application logic.
- **Observer**: Runtime-host continues to emit trace/audit events for each host command, allowing UI replay and backend diagnostics.

## Data Flow

1. The WASM component starts with the user chat payload and workspace metadata.
2. A generic `service.git/git.status` call records workspace provenance.
3. The coordinator agent receives the task and git status, decides `simple`, `standard`, or `deep` complexity using model judgment, and emits a collaboration plan.
4. The planner receives the coordinator output and produces the implementation plan appropriate to that model-authored complexity.
5. The coder receives the coordinator and planner outputs and performs file/tool work through declared Workbench capabilities.
6. The reviewer receives the coordinator, planner, and coder outputs and returns structured review findings or a model-justified lightweight review for simple work.

## Boundary Decision

The application package owns the prompt contract and the agent role choreography. Macaca OS owns only the generic ABI, host-command execution, trace propagation, policy, service routing, and durable event storage. This keeps the OS provider-neutral and lets other applications define different agent collaboration strategies without source changes.

## Logging And Audit

Runtime-host already logs every declared host command with session id, trace id, import, service id, and operation. The Workbench package adds workflow-step metadata to each delegation command so persisted events and diagnostics can identify the active application-owned role without adding OS-level semantics.
