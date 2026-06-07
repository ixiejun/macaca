# Design: Autonomous Execution Envelope

## Context

Macaca OS must execute user-defined scheduled tasks and heartbeat tasks without
requiring users to author typed contracts. The platform also must not let normal
persona context override the source task instruction. The envelope is therefore
a deterministic OS-owned boundary around otherwise natural-language work.

## Goals / Non-Goals

- Goal: preserve the original source instruction as the authoritative delegated
  task.
- Goal: derive minimal completion policy from generic metadata, especially
  `evidence.*` keys.
- Goal: keep wake, dispatch, execution, and evidence states traceable.
- Goal: evaluate Agent Execution results against the compiled completion policy
  instead of using one hardcoded success rule for every autonomous run.
- Non-goal: infer business-domain semantics from arbitrary user text.
- Non-goal: require users to choose tools or write structured contracts.
- Non-goal: introduce application-specific runtime behavior.

## Pattern Choices

- Command: `AgentExecutionCommand` carries the envelope across the service
  boundary.
- Builder: envelope construction normalizes source instruction and metadata in
  one place.
- Strategy: dispatch sources choose source kind while Agent Execution chooses
  how to render and enforce the envelope.
- Specification: completion policy expresses generic evidence requirements.
- Decorator: trace, policy, audit, budget, and resource gates remain service
  boundary concerns.
- Memento: execution results and run histories retain safe metadata for replay.

## Design

Runtime Host compiles scheduled-task and heartbeat dispatches into an
`AutonomousExecutionEnvelope`. The compiler is deterministic: it preserves the
source instruction, labels the source kind, sets task-over-persona priority, and
derives completion policy from generic evidence metadata. If
`evidence.expected_artifact_path` exists, the envelope requires artifact
evidence. Otherwise it requires at least an agent result.

Agent Execution renders the envelope before ordinary delegated context with
language that identifies it as the highest-priority delegated execution
contract. This does not make prompt text the final arbiter of success:
Runtime Host evaluates the result against the envelope completion policy after
execution.

`RequireAgentResult` accepts a completed Agent Execution result only when the
service returns bounded result evidence such as `result_output_hash`.
`RequireArtifact` requires artifact evidence such as `artifact_ref` or
`artifact_digest`. This keeps simple natural-language tasks usable while still
making artifact-declared tasks prove that durable work happened.

## Boundary Notes

The envelope is generic OS data. It does not branch on application names, agent
roles, provider names, domains, chains, symbols, workflows, or file names.
Applications may declare metadata and natural-language instructions; OS services
compile and audit the execution boundary.
