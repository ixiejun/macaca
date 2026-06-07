# WASM 阶段 5：Host Import Service Portal 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to execute this plan task-by-task. 本阶段让 WASM guest 通过受控 host imports 调用 Macaca 系统服务。

## Goal

把 WASM guest imports 接入 Macaca Service Runtime：service call、storage、GenUI render、memory recall、context snapshot、plugin hook、task/session event、payment/web3 optional import。所有 host import 都必须变成 typed Command，经过 trace、policy、capability、service registry 和 sanitized result。

## Scope

本阶段覆盖：

- host import bridge contract。
- import command taxonomy。
- permission/capability check。
- service runtime dispatch adapter。
- sanitized result/error mapping。
- host import trace/audit/log。

本阶段不覆盖：

- 每个系统服务的真实业务 provider 扩展。
- payment/web3 真实链上执行。
- raw guest IO。

## Required Governance Inputs

- `2026-05-12-wasm-runtime-provider-contract-plan.md`
- `2026-05-12-wasm-sandbox-resource-governance-plan.md`
- `docs/superpowers/plans/2026-05-08-s5-llm-memory-context-serviceization-plan.md`
- `docs/superpowers/plans/2026-05-09-s6-driver-skill-mcp-serviceization-plan.md`
- `docs/superpowers/plans/2026-05-11-plugin-service-enrichment-plan.md`

## Architecture Decision

采用 Command + Bridge + Proxy + Chain of Responsibility + Observer：

- Command：每个 import 都转为 bounded typed command。
- Bridge：guest import ABI 与 Macaca service runtime 解耦。
- Proxy：guest SDK 看到的是 service proxy，实际调用走 host bridge。
- Chain of Responsibility：trace -> permission -> capability -> policy -> service availability -> payload bound -> dispatch。
- Observer：import requested/allowed/denied/completed/failed 都写 sanitized trace/log。

## Proposed OpenSpec Change

建议 change id：

- `add-wasm-host-import-service-portal`

建议 specs：

- `wasm-host-imports`
- `wasm-service-portal`
- `wasm-host-import-audit`
- `wasm-host-import-error-taxonomy`

提案必须声明：

- guest 不得直接调用 provider/backend。
- host import 不得绕过 ServiceRuntime、policy facade、capability registry。
- mutating imports 必须 trace-required。
- read-only imports 也必须 bounded 和 sanitized。

## Implementation Slices

### Slice 5.1：Impact 与 service bridge 审计

- [ ] 审计 ApplicationHostCommand、ServiceRuntime、SDK service clients、Plugin/GenUI/Memory/Context service contracts。
- [ ] 对 host command/service runtime 相关 symbols 运行 GitNexus impact。
- [ ] 确认不把 system service implementation 拉入 WASM provider contract。

### Slice 5.2：OpenSpec host imports

- [ ] 定义 import categories 和 command schema。
- [ ] 定义 permission/capability mapping。
- [ ] 定义 allowed/denied/unavailable/error reason code。
- [ ] 定义 sanitized audit event。

### Slice 5.3：Host import bridge

- [ ] 增加 `WasmHostImportBridge` implementation。
- [ ] 增加 command builder 和 validator。
- [ ] 增加 payload bound check。
- [ ] 增加 policy/capability admission hook。
- [ ] 所有新增 Rust 代码写详细英文注释。

### Slice 5.4：Service runtime adapter

- [ ] 将 import command 转发到 provider-neutral ServiceRuntime command。
- [ ] 对 service unavailable 返回 structured unavailable。
- [ ] 对 policy denied 返回 fail-closed。
- [ ] 对 success result 做 redaction/bounding。

### Slice 5.5：Guest import integration

- [ ] default provider 调用 host import bridge。
- [ ] import trace 与 WASM execution trace 串联。
- [ ] 防止 reentrant unbounded call 和无限 payload。

### Slice 5.6：测试

- [ ] service call allowed/denied 测试。
- [ ] missing trace denied 测试。
- [ ] missing capability denied 测试。
- [ ] service unavailable structured result 测试。
- [ ] sanitized result 不含 raw prompt/raw payload 测试。

## Validation

- `cargo test -p macaca-runtime-host wasm_host_import`
- `cargo test -p macaca-sdk wasm_guest_import_contract`
- `cargo test -p macaca-integration-tests application_platform_contracts`
- `openspec validate add-wasm-host-import-service-portal --strict`
- `npx gitnexus detect-changes -r agent`

## Risks

- 风险：host imports 变成宏服务网关。缓解：只做 bridge/command，不拥有服务业务实现。
- 风险：import payload 泄露。缓解：bounded DTO + redaction。
- 风险：循环调用或长链阻塞。缓解：timeout、depth、budget、cancellation guard。
