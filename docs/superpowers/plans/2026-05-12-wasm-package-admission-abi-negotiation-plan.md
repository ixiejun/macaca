# WASM 阶段 2：Package Admission 与 ABI Negotiation 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to execute this plan task-by-task. 本阶段把 WASM artifact、WIT ABI、manifest、ability admission 纳入工业级控制面。

## Goal

完善 Application Framework 控制面，使 WASM application package 能被稳定、安全、可审计地 admission：校验 artifact reference、hash/signature metadata、WIT/ABI version、required imports、exported abilities、permission/service dependency、resource profile 和 compatibility matrix。

## Scope

本阶段覆盖：

- WASM artifact descriptor 与 package admission DTO。
- WIT/ABI semantic version negotiation。
- required imports / exported abilities 校验。
- runtime provider capability 与 app requirement 匹配。
- package admission report 与 compatibility report。
- OpenSpec 增量规范。

本阶段不覆盖：

- 真实编译/实例化。
- artifact 下载、Store 分发、签名链全量验证。
- host import 实际 service dispatch。
- guest SDK 生成。

## Required Governance Inputs

- `2026-05-12-wasm-runtime-provider-contract-plan.md`
- `docs/superpowers/plans/2026-05-12-industrial-wasm-application-runtime-brainstorm.md`
- `macaca/application-wit/macaca-application.wit`
- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/design_patterns.md`

## Architecture Decision

采用 Specification + Visitor + Memento + Adapter：

- Specification：manifest、artifact、ABI、permission、service dependency、resource limits 分成可组合 admission specs。
- Visitor：certification/admission checker 遍历 manifest、ability、artifact、imports、exports。
- Memento：admission report 和 compatibility report 是可审计快照，不含 raw artifact。
- Adapter：legacy WASM metadata-only descriptor 适配到新的 artifact admission model。
- Observer：每个 admission decision 记录 sanitized trace/log。

## Proposed OpenSpec Change

建议 change id：

- `add-wasm-package-admission-abi-negotiation`

建议产物：

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/wasm-package-admission/spec.md`
- `specs/wasm-abi-negotiation/spec.md`
- `specs/wasm-compatibility-report/spec.md`

提案必须声明：

- raw WASM bytes 不进入 manifest、metadata API、trace 或 logs。
- artifact 必须通过 id/hash/signature metadata 引用。
- ABI mismatch 必须 fail-closed，不允许 fallback 到非 WASM 特权路径。
- admission report 必须可被 Web/CLI sanitized metadata 展示。

## Implementation Slices

### Slice 2.1：Impact 与当前控制面审计

- [ ] 阅读 `ApplicationManifestV1`、ability descriptor、WASM descriptor、compatibility checker。
- [ ] 对计划修改的 checker/adapter symbols 运行 GitNexus impact。
- [ ] 分类当前 YAML adapter 与 WASM skeleton adapter 的交界。

### Slice 2.2：OpenSpec admission 规范

- [ ] 创建 WASM artifact descriptor 规范。
- [ ] 创建 ABI negotiation 规范。
- [ ] 创建 compatibility/admission report 规范。
- [ ] 明确所有失败路径必须有 traceable reason code。

### Slice 2.3：Artifact 与 ABI DTO

- [ ] 增加 `WasmComponentArtifactDescriptor`、`WasmArtifactDigest`、`WasmAbiRequirement`。
- [ ] 增加 `WasmImportRequirement`、`WasmExportDeclaration`、`WasmAbiNegotiationResult`。
- [ ] 增加 deterministic sorting 和 stable serialization。
- [ ] 新增 Rust 代码必须有详细英文注释。

### Slice 2.4：Admission specifications

- [ ] 实现 artifact reference spec。
- [ ] 实现 ABI version compatibility spec。
- [ ] 实现 required imports vs declared permissions spec。
- [ ] 实现 runtime capability matching spec。
- [ ] 实现 sanitized admission report projector。

### Slice 2.5：Legacy adapter

- [ ] 保留 metadata-only skeleton 语义。
- [ ] 把旧 descriptor 标记 deprecated 或 legacy adapter，不删除。
- [ ] legacy path 输出新 admission report，方便迁移追踪。

### Slice 2.6：测试

- [ ] ABI match/mismatch 测试。
- [ ] missing artifact digest 测试。
- [ ] missing permission for import 测试。
- [ ] sanitized report 不泄露 raw manifest/raw bytes 测试。

## Validation

- `cargo test -p macaca-proto wasm_abi`
- `cargo test -p macaca-app wasm_admission`
- `cargo test -p macaca-sdk application_testkit`
- `openspec validate add-wasm-package-admission-abi-negotiation --strict`
- `npx gitnexus detect-changes -r agent`

## Risks

- 风险：ABI negotiation 太复杂。缓解：先支持 semantic version + capability flags，保留 extension points。
- 风险：admission 与 certification 重复。缓解：共享 Specification 和 report DTO。
- 风险：legacy skeleton 被误认为真实 runtime。缓解：status 明确 `metadata_only` / `runtime_unavailable`。
