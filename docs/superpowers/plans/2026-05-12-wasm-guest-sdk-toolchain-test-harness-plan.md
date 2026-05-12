# WASM 阶段 7：Guest SDK / Toolchain / Local Test Harness 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to execute this plan task-by-task. 本阶段让第三方开发者真正能开发、测试、打包 WASM application。

## Goal

提供面向应用开发者的 WASM guest SDK、WIT binding workflow、manifest/package builder integration、local test harness、mock host imports、示例应用和开发者文档。目标是让 WASM application 成为可开发、可测试、可发布的一等 application 类型，而不是只存在 runtime 内部。

## Scope

本阶段覆盖：

- Rust guest SDK scaffold。
- WIT binding generation workflow。
- guest-side service proxy。
- local mock host import harness。
- package builder integration。
- example apps。
- SDK contract tests。

本阶段不覆盖：

- 全语言 SDK 完整实现。
- IDE 插件。
- Store publish pipeline。

## Required Governance Inputs

- `2026-05-12-wasm-host-import-service-portal-plan.md`
- `docs/superpowers/plans/2026-05-12-application-platform-option-e-plan.md`
- `macaca/application-wit/macaca-application.wit`
- `macaca/docs/design_patterns.md`

## Architecture Decision

采用 Facade + Proxy + Builder + Adapter + Test Double：

- Facade：guest SDK 暴露简单 Application/Ability/Service API。
- Proxy：guest service client 只是 host import proxy。
- Builder：manifest、ability、permission、service dependency、package artifact 由 SDK builder 生成。
- Adapter：WIT bindings 适配到 SDK facade。
- Test Double：local mock host imports 支持无 Macaca runtime 的 contract tests。

## Proposed OpenSpec Change

建议 change id：

- `add-wasm-guest-sdk-toolchain-test-harness`

建议 specs：

- `wasm-guest-sdk`
- `wasm-toolchain`
- `wasm-local-test-harness`
- `wasm-example-apps`

提案必须声明：

- guest SDK 只能依赖 WIT/ABI 和 provider-neutral contracts。
- guest SDK 不得嵌入 provider name、gateway name、driver name、workflow name 或业务名称。
- local mock host 必须与 real host import contract 共享 test fixtures。

## Implementation Slices

### Slice 7.1：Impact 与 SDK 边界审计

- [ ] 审计 `macaca-sdk` ApplicationKit/AbilityKit/TestKit。
- [ ] 审计 WIT schema 与现有 ABI DTO。
- [ ] 对将修改的 SDK symbols 运行 GitNexus impact。

### Slice 7.2：OpenSpec SDK/toolchain

- [ ] 定义 guest SDK API surface。
- [ ] 定义 WIT binding generation workflow。
- [ ] 定义 local harness behavior。
- [ ] 定义 examples 和 contract tests 要求。

### Slice 7.3：Rust guest SDK scaffold

- [ ] 新增 guest SDK scaffold 或模板目录。
- [ ] 提供 Application/Ability lifecycle facade。
- [ ] 提供 service proxy、storage proxy、GenUI proxy、memory/context proxy。
- [ ] 所有 Rust 代码写详细英文注释。

### Slice 7.4：Toolchain integration

- [ ] SDK package builder 支持 WASM artifact descriptor。
- [ ] manifest builder 支持 WASM ability。
- [ ] testkit 支持 ABI/import/permission consistency check。
- [ ] 添加 deterministic fixture generation。

### Slice 7.5：Local test harness

- [ ] mock host imports 支持 allowed/denied/unavailable/success。
- [ ] harness 生成 sanitized trace。
- [ ] harness 与 runtime host import contract 共用 DTO。

### Slice 7.6：Examples

- [ ] headless WASM app fixture。
- [ ] GenUI render WASM app fixture。
- [ ] memory/context import fixture。
- [ ] service unavailable fixture。

## Validation

- `cargo test -p macaca-sdk wasm_guest_sdk`
- `cargo test -p macaca-sdk application_testkit`
- `cargo test -p macaca-integration-tests application_platform_contracts`
- `openspec validate add-wasm-guest-sdk-toolchain-test-harness --strict`
- `npx gitnexus detect-changes -r agent`

## Risks

- 风险：SDK 与 runtime contract 漂移。缓解：共享 DTO 和 contract tests。
- 风险：只支持 Rust 导致生态受限。缓解：Rust 作为第一语言 scaffold，WIT workflow 为其他语言保留。
- 风险：mock host 与真实 host 行为不一致。缓解：共用 fixture 和 error taxonomy。
