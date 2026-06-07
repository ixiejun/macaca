# Tasks: Add Autonomous Execution Envelope

## 1. Contracts
- [x] 1.1 Add provider-neutral envelope, mode, priority, and completion-policy DTOs to Agent Execution contracts.
- [x] 1.2 Preserve backward compatibility with existing Agent Execution command constructors and serialization.

## 2. Runtime Dispatch
- [x] 2.1 Compile scheduled-agent-task dispatches into envelopes before calling Agent Execution.
- [x] 2.2 Compile heartbeat-agent dispatches into envelopes before calling Agent Execution.
- [x] 2.3 Log envelope source kind and completion policy without raw prompt or raw provider output.

## 3. Agent Execution
- [x] 3.1 Render the envelope as the highest-priority delegated execution contract.
- [x] 3.2 Keep generic evidence validation as the post-execution success gate.

## 4. Validation
- [x] 4.1 Add failing tests for envelope compilation and rendering, then implement.
- [x] 4.2 Run focused Rust tests.
- [x] 4.3 Run `openspec validate add-autonomous-execution-envelope --strict`.
- [x] 4.4 Run GitNexus detect changes.

## 5. Policy-Aware Completion
- [x] 5.1 Add a policy-aware Agent Execution evidence gate for envelope completion policies.
- [x] 5.2 Route scheduled-agent-task dispatch result classification through the compiled policy.
- [x] 5.3 Route heartbeat-agent dispatch result classification through the compiled policy.
- [x] 5.4 Validate policy-aware result gates and OpenSpec.
