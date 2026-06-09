## 1. Proposal Approval

- [x] 1.1 Review this OpenSpec proposal, design, and delta spec with the architecture constitution.
- [x] 1.2 Confirm that the 25 listed gaps are framework-owned contracts or delegated framework ports, not concrete service ownership.
- [x] 1.3 Approve implementation before any Rust source changes begin.

## 2. Agent Runtime And Middleware Contracts

- [x] 2.1 Add pure event-stream agent contracts for callable, streamable, and observable agents; make reply/final-message helpers projections only.
- [x] 2.2 Replace old hook-primary behavior with middleware-first ABI and migration annotations for any consumer-facing hook APIs that cannot be removed immediately.
- [x] 2.3 Add UserAgent, StreamUserInput, and HITL input agent contracts with explicit suspend/resume state.
- [x] 2.4 Add StreamOptions and structured-output-capable agent contracts.
- [x] 2.5 Add built-in middleware contracts for tracing, task reminders, provider-neutral system prompt processing, and RAG/context retrieval.

## 3. Tools, Models, MCP, And Protocol Contracts

- [x] 3.1 Add ToolBase-first contracts: ToolBase, ToolSpec, ToolInvocation, ToolResult, and old toolkit non-primary bridge notes where needed.
- [x] 3.2 Add provider-neutral tool context injection contracts.
- [x] 3.3 Add full tool suspend/external execution state machine with correlation, idempotency, resume, stale-result rejection, and audit evidence.
- [x] 3.4 Add model formatter/parser parity contracts for AgentScope Java 2.0-supported formatter families while keeping concrete clients delegated.
- [x] 3.5 Add model transport command/result/error/stream DTO contracts for HTTP/WebSocket-like transports without concrete transport implementation.
- [x] 3.6 Add model exception taxonomy for auth, bad request, rate limit, not found, permission, internal, timeout, unavailable, and provider failure states.
- [x] 3.7 Add MCP content conversion contracts into ContentBlock and ToolResult mappings.
- [x] 3.8 Add Agent Protocol typed projection from AgentEvent.

## 4. Harness Contracts

- [x] 4.1 Add two-layer workspace read contract: filesystem-first plus authorized local fallback.
- [x] 4.2 Add filesystem spec DTOs for local, remote, sandbox, composite, overlay, and baked filesystem shapes.
- [x] 4.3 Add backend-neutral sandbox spec/state DTOs for Docker, E2B, Kubernetes, Daytona, AgentRun, and unavailable backends.
- [x] 4.4 Add session tree, freshness, restore, and checkpoint contracts.
- [x] 4.5 Add memory maintenance contracts: MemoryConsolidator, schedule/result DTOs, and session memory search ports.
- [x] 4.6 Add skill runtime contracts: resources, lazy resources, skill load tool, visibility, conflict resolution, promotion, and audit DTOs.
- [x] 4.7 Add subagent dynamic spec contracts: spec generator, workspace mode, remote stub contract, stream forwarding choice, and nested child event projection.
- [x] 4.8 Add PlanMode tool contracts for plan enter/exit, three plan tools, HITL exit semantics, and task-service mutation delegation.

## 5. Capability Evidence And Provider Snapshots

- [x] 5.1 Replace broad availability claims with evidence-backed capability matrix statuses: equivalent, contract-only, delegated-verified, delegated-unverified, missing, unsupported-by-policy.
- [x] 5.2 Add capability evidence refs, delegation targets, test coverage refs, known limitations, and snapshot serialization.
- [x] 5.3 Add structured unavailable/null-object provider behavior for absent optional services.
- [x] 5.4 Add health/snapshot tests that prove no unsupported capability is reported as available.

## 6. Boundary, Observability, And License Gates

- [x] 6.1 Add dependency-boundary tests proving framework does not import concrete provider implementations or presentation shells.
- [x] 6.2 Add trace/audit replay tests for agent run start/end, middleware, model request, tool request, suspend/resume, denial, failure, and completion.
- [x] 6.3 Add sanitized logging checks for secrets, raw prompts, raw provider payloads, manifests, package bytes, WASM bytes, private keys, credentials, signatures, and unbounded output.
- [x] 6.4 Add naming gates rejecting AgentScope 1.0 leftovers, version-suffixed canonical names, and internal legacy/compat/deprecated runtime fallbacks.
- [x] 6.5 Add Apache-2.0 SPDX/provenance header checks for AgentScope Java 2.0-derived or closely adapted framework source files.
- [x] 6.6 Add English-comment review checks for non-obvious framework state transitions, adapters, middleware, and side-effect boundaries.

## 7. Verification

- [x] 7.1 Run `openspec validate add-framework-agentscope2-contract-parity --strict`.
- [x] 7.2 Run targeted framework unit and contract tests.
- [x] 7.3 Run Macaca dependency-boundary gates.
- [x] 7.4 Run `/api/chat/v2`, session recovery, YAML, WASM, GenUI, trace replay, and optional-provider unavailable regression scenarios.
- [x] 7.5 Run GitNexus detect changes before commit; record HIGH/CRITICAL findings as memo-only for this refactor unless they reveal an actual boundary violation.
