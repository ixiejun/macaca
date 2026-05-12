# WASM 阶段 1：Runtime Provider Contract 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to execute this plan task-by-task. 本计划只覆盖工业级 WASM Application Runtime 的 provider-neutral contract，不引入真实 engine 执行。

## Goal

为 Option D 分层工业级 WASM Application Runtime 建立可插拔执行面契约：`WasmApplicationRuntimeProvider`、engine capability、execution profile、runtime session、diagnostics、host import bridge contract。完成后，Application Framework、SDK、Runtime Host 和 future provider 都通过稳定 contract 交互，不能把 Wasmtime 或任何具体 engine 泄漏到公共接口。

## Scope

本阶段覆盖：

- provider-neutral WASM runtime trait / DTO / command / result。
- engine capability 与 deployment profile 表达。
- runtime provider registry 所需的 descriptor / availability / diagnostics。
- execution session 的最小状态、trace、resource envelope。
- unavailable provider 作为 Null Object fallback。
- OpenSpec 增量规范。

本阶段不覆盖：

- 真实 Wasmtime / WasmEdge / Wasmer 集成。
- WASI 资源授权细节。
- guest SDK 代码生成。
- out-of-process runtime 的真实 IPC。
- Store certification 全量测试。

## Required Governance Inputs

- `docs/superpowers/plans/2026-05-12-industrial-wasm-application-runtime-brainstorm.md`
- `docs/superpowers/plans/2026-05-12-application-platform-option-e-plan.md`
- `openspec/changes/add-wasm-component-application-abi-skeleton/design.md`
- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/design_patterns.md`

## Architecture Decision

采用 Bridge + Abstract Factory + Strategy + Null Object：

- Bridge：WASM guest/engine 与 Macaca service runtime 通过 provider-neutral host bridge 隔离。
- Abstract Factory：runtime provider factory 根据 runtime kind、deployment profile 和 policy 创建 provider/session。
- Strategy：engine selection、compile mode、instantiation mode、diagnostics detail level 都是可替换策略。
- Null Object：未安装 runtime provider 时返回 structured unavailable，不 panic、不绕过权限。
- Observer：availability、provider selection、session creation 必须发出 trace/log。
- Specification：contract constructor 校验 trace、application id、ability id、artifact reference、runtime profile。

公共 contract 必须只依赖 provider-neutral DTO，禁止出现具体 engine 类型、provider 名称分支或业务硬编码。

## Proposed OpenSpec Change

建议 change id：

- `add-wasm-runtime-provider-contract`

建议产物：

- `openspec/changes/add-wasm-runtime-provider-contract/proposal.md`
- `openspec/changes/add-wasm-runtime-provider-contract/design.md`
- `openspec/changes/add-wasm-runtime-provider-contract/tasks.md`
- `openspec/changes/add-wasm-runtime-provider-contract/specs/wasm-runtime-provider/spec.md`
- `openspec/changes/add-wasm-runtime-provider-contract/specs/wasm-runtime-diagnostics/spec.md`

提案必须声明：

- WASM runtime provider 属于 execution plane，不属于 Kernel 或 Application Framework。
- Application Framework 只能依赖 runtime provider contract 和 descriptor。
- SDK 只能生成/消费 provider-neutral contract，不构造 runtime provider。
- 缺失 provider 必须 fail-closed/unavailable，并带 traceable reason。
- diagnostics 必须脱敏，不能包含 raw WASM bytes、raw payload、raw manifest、secret、env、API key。

## Implementation Slices

### Slice 1.1：Impact 与边界审计

- [ ] 阅读现有 WASM skeleton、Application ABI、Application Host Factory、ServiceRuntime。
- [ ] 使用 GitNexus 对将要修改的 symbols 做 upstream impact。
- [ ] 标注当前 unavailable host 语义，确认保留原行为作为 fallback。
- [ ] 确认新增 contract 放置在 foundation/application/runtime 的边界是否符合 allowlist。

### Slice 1.2：OpenSpec contract

- [ ] 创建 proposal/design/tasks/spec。
- [ ] 明确 provider registry、runtime descriptor、execution profile、diagnostics、session lifecycle 的 MUST/SHALL。
- [ ] 明确公共 contract 不允许暴露具体 engine 类型。
- [ ] 明确 trace-required 和 sanitized diagnostics。

### Slice 1.3：Provider-neutral DTO 与 trait

- [ ] 增加 `WasmRuntimeProviderDescriptor`、`WasmEngineCapabilities`、`WasmExecutionProfile`。
- [ ] 增加 `WasmRuntimeAvailability`、`WasmRuntimeUnavailableReason`、`WasmRuntimeDiagnostics`。
- [ ] 增加 `WasmApplicationRuntimeProvider` trait 和 `WasmExecutionSession` trait。
- [ ] 所有新增 Rust 代码添加详细英文注释，说明功能、运行原理、边界和不变量。

### Slice 1.4：Unavailable provider

- [ ] 实现 unavailable provider，保留现有 skeleton 的 fail-closed 语义。
- [ ] unavailable result 必须包含 trace id、runtime kind、reason code、sanitized diagnostics。
- [ ] 记录关键日志：provider selected、provider unavailable、session rejected。

### Slice 1.5：Contract tests

- [ ] 测试 descriptor deterministic serialization。
- [ ] 测试 unavailable provider 不执行 guest。
- [ ] 测试 diagnostics 不包含 raw bytes/raw payload。
- [ ] 测试 missing trace 被拒绝或返回 fail-closed。

## Validation

- `cargo test -p macaca-proto wasm_runtime`
- `cargo test -p macaca-runtime-host wasm_runtime_provider`
- `cargo test -p macaca-integration-tests route_c_dependency_boundaries`
- `openspec validate add-wasm-runtime-provider-contract --strict`
- `npx gitnexus detect-changes -r agent`

## Risks

- 风险：trait 过度贴近 Wasmtime。缓解：contract 只使用 Macaca ABI DTO 和 provider-neutral command/result。
- 风险：contract 太抽象导致不可实现。缓解：以 unavailable provider 和后续 default provider 的最小闭环校验。
- 风险：runtime-host 变巨型文件。缓解：provider、descriptor、diagnostics、session、unavailable 分模块，单文件低于 500 行。
