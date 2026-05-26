## 1. Planning Provider

- [ ] 1.1 Add `macaca-runtime-host/src/tool_service_provider.rs`.
- [ ] 1.2 Add `macaca-runtime-host/src/tool_service_provider_state.rs`.
- [ ] 1.3 Add `macaca-runtime-host/src/tool_service_planning.rs`.
- [ ] 1.4 Register `service.tool` with the service runtime.
- [ ] 1.5 Add provider status cache and plan snapshot cache.

## 2. Descriptor Contributors

- [ ] 2.1 Add Driver descriptor contributor.
- [ ] 2.2 Add Skill descriptor contributor.
- [ ] 2.3 Add MCP descriptor contributor.
- [ ] 2.4 Add Memory descriptor contributor.
- [ ] 2.5 Add Task and Scheduler descriptor contributors.
- [ ] 2.6 Add Gateway descriptor contributor.
- [ ] 2.7 Add workspace/runtime tool descriptor contributor.

## 3. Availability And Toolsets

- [ ] 3.1 Add `tool_service_availability.rs`.
- [ ] 3.2 Add availability evaluators for config, secret, auth, env, binary, service health, platform, resource, entitlement, plugin, manifest, agent policy, and session context.
- [ ] 3.3 Add family resolution strategy.
- [ ] 3.4 Add toolset resolution strategy.
- [ ] 3.5 Add conflict detection and stable hidden diagnostics.

## 4. Context And Manifest

- [ ] 4.1 Add compact tool capability provider in `macaca-context`.
- [ ] 4.2 Add report fields for visible, hidden, skipped, and conflicted tool counts.
- [ ] 4.3 Add generic manifest fields for tool families and toolsets.
- [ ] 4.4 Preserve exact `allowed_tools` compatibility.

## 5. Validation

- [ ] 5.1 Add unit tests for visible/hidden planning.
- [ ] 5.2 Add tests for missing auth, missing config, missing binary, missing service, platform mismatch, policy denied, entitlement missing, and name conflicts.
- [ ] 5.3 Add context report tests for tool capability counts.
- [ ] 5.4 Add manifest compatibility tests for exact `allowed_tools`.
- [ ] 5.5 Run `cargo test -p macaca-runtime-host tool_service_planning -- --nocapture`.
- [ ] 5.6 Run `cargo test -p macaca-context -- --nocapture`.
- [ ] 5.7 Run `openspec validate add-tool-capability-planning-service --strict`.
- [ ] 5.8 Run `git diff --check`.

## 6. Governance Notes

- [ ] 6.1 Confirm provider lifecycle remains in owning services.
- [ ] 6.2 Confirm context receives compact bounded indexes only.
- [ ] 6.3 Record GitNexus `CRITICAL` and `HIGH` warnings as notes per user instruction.
