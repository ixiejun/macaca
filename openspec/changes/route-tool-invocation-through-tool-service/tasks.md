## 1. Invocation Service

- [x] 1.1 Add `tool_service_invocation.rs`.
- [x] 1.2 Add descriptor route lookup by stable route metadata.
- [x] 1.3 Route MCP tools to `SystemMcpClient::invoke_tool`.
- [x] 1.4 Route Skill tools to `SystemSkillClient::invoke_tool`.
- [x] 1.5 Route Driver tools to `SystemDriverClient::invoke_tool`.
- [ ] 1.6 Route Memory, Task, Scheduler, Gateway, Store, and runtime tools to focused services or provider adapters.
- [x] 1.7 Add `tool.invoke.cancel`, `tool.invocation.status`, and `tool.result.get` handling.

## 2. Enforcement Decorators

- [x] 2.1 Add trace decorator.
- [x] 2.2 Add policy decorator.
- [x] 2.3 Add approval decorator.
- [ ] 2.4 Add resource admission decorator.
- [ ] 2.5 Add entitlement and metering decorators.
- [x] 2.6 Add timeout and cancellation decorator.
- [x] 2.7 Add redaction decorator.
- [x] 2.8 Add result budget decorator.

## 3. Framework Toolkit Migration

- [ ] 3.1 Add `macaca-web/src/tool_service_adapter.rs`.
- [x] 3.2 Convert service-owned descriptors into framework tools that call `SystemToolClient`.
- [x] 3.3 Reapply manifest compatibility filtering during transition.
- [x] 3.4 Mark old direct assembly paths compatibility-only or deprecated.
- [x] 3.5 Ensure Web remains a shell adapter and does not own policy or provider lifecycle.

## 4. Results And Audit

- [x] 4.1 Add `tool_service_result.rs`.
- [ ] 4.2 Add `tool_service_audit.rs`.
- [x] 4.3 Normalize small inline results.
- [x] 4.4 Persist oversized or binary results as artifact refs.
- [ ] 4.5 Return background handles for long-running invocations.
- [x] 4.6 Return approval requests before side effects.
- [x] 4.7 Return structured unavailable, unsupported, denied, failed, cancelled, and timeout states.
- [x] 4.8 Emit invocation lifecycle events.

## 5. Validation

- [ ] 5.1 Add invocation routing tests for MCP, Skill, Driver, Memory, Task, Scheduler, Gateway, and runtime tools.
- [x] 5.2 Add policy-denied tests.
- [ ] 5.3 Add missing-scope, missing-trace, unknown-tool, unavailable-provider, and timeout tests.
- [x] 5.4 Add large-result artifact tests.
- [ ] 5.5 Add audit sanitization tests.
- [x] 5.6 Run `cargo test -p macaca-runtime-host tool_service_invocation -- --nocapture`.
- [x] 5.7 Run `cargo test -p macaca-web framework_toolkit -- --nocapture`.
- [x] 5.8 Run `openspec validate route-tool-invocation-through-tool-service --strict`.
- [x] 5.9 Run `git diff --check`.

## 6. Governance Notes

- [x] 6.1 Confirm owning services retain lifecycle and concrete invocation authority.
- [x] 6.2 Confirm Web/CLI/frontend remain shell adapters.
- [x] 6.3 Record GitNexus `CRITICAL` and `HIGH` warnings as notes per user instruction.

### GitNexus Risk Notes

- `service_tool_from_descriptor` impact: `CRITICAL`; direct caller `build_toolkit`; affected processes include `Build_toolkit -> Config`, `Build_toolkit -> Get_app`, `Build_toolkit -> List`, `Build_toolkit -> New`, `Build_toolkit -> ToolGroup`, `Build_toolkit -> AppAgentManifestView`, `Build_toolkit -> RegisteredTool`, and `Build_toolkit -> Allowed_tools`.
- `build_toolkit` impact: `LOW`; direct caller `WebTracedAgentFactory.prepare_agent_parts`.
- `ToolSystemServiceProvider` and `ToolInvokeCommand` were not found in the current GitNexus index before editing; treated as stale-index notes rather than blockers per instruction.
- `gitnexus_detect_changes(scope=all)` after implementation reported `MEDIUM` risk with affected `Serve_web_server` processes.
