## 1. Invocation Service

- [ ] 1.1 Add `tool_service_invocation.rs`.
- [ ] 1.2 Add descriptor route lookup by stable route metadata.
- [ ] 1.3 Route MCP tools to `SystemMcpClient::invoke_tool`.
- [ ] 1.4 Route Skill tools to `SystemSkillClient::invoke_tool`.
- [ ] 1.5 Route Driver tools to `SystemDriverClient::invoke_tool`.
- [ ] 1.6 Route Memory, Task, Scheduler, Gateway, Store, and runtime tools to focused services or provider adapters.
- [ ] 1.7 Add `tool.invoke.cancel`, `tool.invocation.status`, and `tool.result.get` handling.

## 2. Enforcement Decorators

- [ ] 2.1 Add trace decorator.
- [ ] 2.2 Add policy decorator.
- [ ] 2.3 Add approval decorator.
- [ ] 2.4 Add resource admission decorator.
- [ ] 2.5 Add entitlement and metering decorators.
- [ ] 2.6 Add timeout and cancellation decorator.
- [ ] 2.7 Add redaction decorator.
- [ ] 2.8 Add result budget decorator.

## 3. Framework Toolkit Migration

- [ ] 3.1 Add `macaca-web/src/tool_service_adapter.rs`.
- [ ] 3.2 Convert `ToolPlanEntry` into framework tools that call `SystemToolClient`.
- [ ] 3.3 Reapply manifest compatibility filtering during transition.
- [ ] 3.4 Mark old direct assembly paths compatibility-only or deprecated.
- [ ] 3.5 Ensure Web remains a shell adapter and does not own policy or provider lifecycle.

## 4. Results And Audit

- [ ] 4.1 Add `tool_service_result.rs`.
- [ ] 4.2 Add `tool_service_audit.rs`.
- [ ] 4.3 Normalize small inline results.
- [ ] 4.4 Persist oversized or binary results as artifact refs.
- [ ] 4.5 Return background handles for long-running invocations.
- [ ] 4.6 Return approval requests before side effects.
- [ ] 4.7 Return structured unavailable, unsupported, denied, failed, cancelled, and timeout states.
- [ ] 4.8 Emit invocation lifecycle events.

## 5. Validation

- [ ] 5.1 Add invocation routing tests for MCP, Skill, Driver, Memory, Task, Scheduler, Gateway, and runtime tools.
- [ ] 5.2 Add policy-denied and approval-required tests.
- [ ] 5.3 Add missing-scope, missing-trace, unknown-tool, unavailable-provider, and timeout tests.
- [ ] 5.4 Add large-result artifact tests.
- [ ] 5.5 Add audit sanitization tests.
- [ ] 5.6 Run `cargo test -p macaca-runtime-host tool_service_invocation -- --nocapture`.
- [ ] 5.7 Run `cargo test -p macaca-web framework_toolkit -- --nocapture`.
- [ ] 5.8 Run `openspec validate route-tool-invocation-through-tool-service --strict`.
- [ ] 5.9 Run `git diff --check`.

## 6. Governance Notes

- [ ] 6.1 Confirm owning services retain lifecycle and concrete invocation authority.
- [ ] 6.2 Confirm Web/CLI/frontend remain shell adapters.
- [ ] 6.3 Record GitNexus `CRITICAL` and `HIGH` warnings as notes per user instruction.
