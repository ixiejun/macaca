## 1. Preparation
- [x] 1.1 Run GitNexus impact for `AgentServices` upstream.
- [x] 1.2 Run GitNexus impact for `AgentCapabilitySet` upstream.
- [x] 1.3 Run GitNexus impact for `AgentStateMachine` upstream.
- [x] 1.4 Confirm current `cargo test -p macaca-agent` baseline passes.

## 2. Services primitive boundary
- [x] 2.1 Add `services.rs` with service traits, no-op implementations, `AgentServices`, and `AgentServicesBuilder`.
- [x] 2.2 Keep public re-exports compatible from `agent.rs` and `lib.rs`.
- [x] 2.3 Add tests for builder-provided memory/ipc/persist services and no-op defaults.
- [x] 2.4 Mark legacy direct service construction helpers as deprecated without deleting them.

## 3. Capability primitive boundary
- [x] 3.1 Add `capability.rs` and move `CapabilitySource`, `AgentCapabilityNode`, and `AgentCapabilitySet` there.
- [x] 3.2 Add read-only helpers: `is_empty`, `len`, `nodes`, `sources`, `from_source`.
- [x] 3.3 Update `basic.rs` to consume capability primitives from the new module.
- [x] 3.4 Add tests proving flattened legacy capability output is unchanged.
- [x] 3.5 Mark legacy BasicAgent constructor helpers as deprecated without deleting them.

## 4. Lifecycle primitive boundary
- [x] 4.1 Add `lifecycle.rs` with `AgentTransitionReason`, `AgentLifecyclePolicy`, `DefaultAgentLifecyclePolicy`, and `AgentLifecycleTransition`.
- [x] 4.2 Keep `AgentStateMachine` API compatible and delegate to lifecycle primitives.
- [x] 4.3 Add `AgentStateMachine::can_transition_to` as additive preflight.
- [x] 4.4 Add transition table tests for preflight and transition behavior equivalence.
- [x] 4.5 Mark legacy state-machine construction helpers as deprecated without deleting them.

## 5. Verification
- [x] 5.1 Run `cargo fmt`.
- [x] 5.2 Run `cargo test -p macaca-agent -- --nocapture`.
- [x] 5.3 Run `cargo check -p macaca-agent -p macaca-framework -p macaca-sdk -p macaca-kernel -p macaca-web`.
- [x] 5.4 Run deprecated/API containment grep for old imports and confirm compatibility.
- [x] 5.5 Run `openspec validate refactor-macaca-agent-primitive-boundaries --strict`.
- [x] 5.6 Run `gitnexus_detect_changes(scope: "all")` before commit.
