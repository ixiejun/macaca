# WASM 阶段 8：Certification / Fixtures / Hardened Provider Contract 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to execute this plan task-by-task. 本阶段把 WASM application 支持收束为可认证、可回归、可面向生态开放的工业级能力。

## Goal

建立 WASM application certification / conformance 体系，并定义 hardened out-of-process provider contract。完成后，Macaca 能对第三方 WASM application 做 ABI、安全、权限、资源、host import、lifecycle、observability、package compatibility 的认证测试，同时为企业级隔离执行保留可替换 provider 边界。

## Scope

本阶段覆盖：

- certification specification。
- conformance fixtures。
- negative security tests。
- sanitized certification report。
- hardened provider contract。
- store/admission integration hooks。
- regression matrix 更新。

本阶段不覆盖：

- 真实 out-of-process runtime daemon 的完整实现。
- Store 商业审核流程。
- 第三方语言 SDK 全量生态。

## Required Governance Inputs

- `2026-05-12-wasm-runtime-provider-contract-plan.md`
- `2026-05-12-wasm-package-admission-abi-negotiation-plan.md`
- `2026-05-12-wasm-sandbox-resource-governance-plan.md`
- `2026-05-12-wasm-host-import-service-portal-plan.md`
- `2026-05-12-wasm-lifecycle-state-checkpoint-plan.md`
- `2026-05-12-wasm-guest-sdk-toolchain-test-harness-plan.md`
- `macaca/docs/route-c-regression-matrix.md`
- `macaca/docs/route-c-architecture-governance.md`

## Architecture Decision

采用 Specification + Visitor + Memento + Template Method + Adapter：

- Specification：certification checks 复用 admission/resource/ABI/host import/lifecycle specs。
- Visitor：checker 遍历 package、manifest、ability、artifact、imports、resource policy、fixtures。
- Memento：certification report 是 sanitized immutable artifact。
- Template Method：不同 certification profile 复用同一测试骨架。
- Adapter：default provider、unavailable provider、hardened provider 都通过同一 conformance adapter 测试。

## Proposed OpenSpec Change

建议 change id：

- `add-wasm-certification-fixtures-hardened-provider-contract`

建议 specs：

- `wasm-certification`
- `wasm-conformance-fixtures`
- `wasm-security-negative-tests`
- `wasm-hardened-provider-contract`
- `wasm-regression-matrix`

提案必须声明：

- 没有通过 certification 的 WASM package 不能被标记为 industrial-ready。
- certification report 不得包含 raw bytes、raw manifest、raw guest payload、secret、env、API key。
- hardened provider contract 必须与 default provider 共享 provider-neutral API。
- out-of-process 是 deployment profile，不是新 application semantics。

## Implementation Slices

### Slice 8.1：Impact 与认证覆盖审计

- [ ] 审计现有 application platform certification fixtures。
- [ ] 审计 route-c regression matrix。
- [ ] 对 certification/testkit/provider registry symbols 运行 GitNexus impact。

### Slice 8.2：OpenSpec certification

- [ ] 定义 certification profiles：dev、default、hardened。
- [ ] 定义 ABI/resource/import/lifecycle/observability/security check requirements。
- [ ] 定义 certification report schema。
- [ ] 定义 hardened provider contract。

### Slice 8.3：Certification runner

- [ ] 实现 certification runner 或扩展 existing Application TestKit。
- [ ] 使用 Visitor 遍历 app package。
- [ ] 使用 Specification 执行 checks。
- [ ] 输出 sanitized memento report。
- [ ] 所有 Rust 代码写详细英文注释。

### Slice 8.4：Conformance fixtures

- [ ] 添加 valid minimal WASM app fixture。
- [ ] 添加 GenUI render fixture。
- [ ] 添加 host import permission fixture。
- [ ] 添加 resource exhausted fixture。
- [ ] 添加 ABI mismatch fixture。
- [ ] 添加 unavailable provider fixture。

### Slice 8.5：Security negative tests

- [ ] raw env access denied。
- [ ] raw filesystem denied。
- [ ] raw network denied。
- [ ] missing trace denied。
- [ ] missing capability denied。
- [ ] oversized payload denied。
- [ ] timeout/resource exhaustion denied。

### Slice 8.6：Hardened provider contract

- [ ] 定义 out-of-process request/response envelope。
- [ ] 定义 trace propagation、cancellation、backpressure、timeout、diagnostics。
- [ ] 不实现真实 daemon，仅实现 contract/mock adapter。
- [ ] 确保 default provider 与 hardened contract 可共享 certification tests。

### Slice 8.7：治理文档与回归矩阵

- [ ] 更新 Route C governance，说明 WASM runtime ownership。
- [ ] 更新 regression matrix，加入 WASM certification gates。
- [ ] 更新 docs，说明工业级 WASM app 的开发、认证、部署路径。

## Validation

- `cargo test -p macaca-sdk wasm_certification`
- `cargo test -p macaca-integration-tests wasm_application_certification`
- `cargo test -p macaca-integration-tests route_c_dependency_boundaries`
- `openspec validate add-wasm-certification-fixtures-hardened-provider-contract --strict`
- `npx gitnexus detect-changes -r agent`

## Risks

- 风险：认证只测 happy path。缓解：必须包含 negative security fixtures。
- 风险：hardened provider 变成第二套语义。缓解：共享 provider-neutral contract 和 conformance tests。
- 风险：报告泄露敏感内容。缓解：report schema 只允许 bounded sanitized fields。
