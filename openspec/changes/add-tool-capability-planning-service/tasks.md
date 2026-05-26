## 1. Planning Provider

- [x] 1.1 Add `macaca-runtime-host/src/tool_service_provider.rs`.
- [x] 1.2 Add `macaca-runtime-host/src/tool_service_provider_state.rs`.
- [x] 1.3 Add `macaca-runtime-host/src/tool_service_planning.rs`.
- [x] 1.4 Register `service.tool` with the service runtime.
- [x] 1.5 Add provider status cache and plan snapshot cache.

## 2. Descriptor Contributors

- [x] 2.1 Add Driver descriptor contributor through the shared `CapabilityToolDescriptorContributor` adapter.
- [x] 2.2 Add Skill descriptor contributor through the shared `CapabilityToolDescriptorContributor` adapter.
- [x] 2.3 Add MCP descriptor contributor through the shared `CapabilityToolDescriptorContributor` adapter.
- [x] 2.4 Add Memory descriptor contributor through the shared `CapabilityToolDescriptorContributor` adapter.
- [x] 2.5 Add Task and Scheduler descriptor contributors through the shared `CapabilityToolDescriptorContributor` adapter.
- [x] 2.6 Add Gateway descriptor contributor through the shared `CapabilityToolDescriptorContributor` adapter.
- [x] 2.7 Add workspace/runtime tool descriptor contributor through the shared `CapabilityToolDescriptorContributor` adapter.

## 3. Availability And Toolsets

- [x] 3.1 Add `tool_service_availability.rs`.
- [x] 3.2 Add availability evaluators for config, secret, auth, env, binary, service health, platform, resource, entitlement, plugin, manifest, agent policy, and session context.
- [x] 3.3 Add family resolution strategy.
- [x] 3.4 Add toolset resolution strategy.
- [x] 3.5 Add conflict detection and stable hidden diagnostics.

## 4. Context And Manifest

- [x] 4.1 Add compact tool capability provider in `macaca-context`.
- [x] 4.2 Add report fields for visible, hidden, skipped, and conflicted tool counts.
- [x] 4.3 Add generic manifest fields for tool families and toolsets.
- [x] 4.4 Preserve exact `allowed_tools` compatibility.

## 5. Validation

- [x] 5.1 Add unit tests for visible/hidden planning.
- [x] 5.2 Add tests for missing auth, missing config, missing binary, missing service, platform mismatch, policy denied, entitlement missing, and name conflicts.
- [x] 5.3 Add context report tests for tool capability counts.
- [x] 5.4 Add manifest compatibility tests for exact `allowed_tools`.
- [x] 5.5 Run `cargo test -p macaca-runtime-host tool_service_planning -- --nocapture`.
- [x] 5.6 Run `cargo test -p macaca-context -- --nocapture`.
- [x] 5.7 Run `openspec validate add-tool-capability-planning-service --strict`.
- [x] 5.8 Run `git diff --check`.

## 6. Governance Notes

- [x] 6.1 Confirm provider lifecycle remains in owning services.
- [x] 6.2 Confirm context receives compact bounded indexes only.
- [x] 6.3 Record GitNexus `CRITICAL` and `HIGH` warnings as notes per user instruction.
