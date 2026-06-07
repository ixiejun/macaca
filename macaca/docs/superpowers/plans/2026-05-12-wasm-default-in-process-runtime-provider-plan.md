# WASM 阶段 3：Default In-Process Runtime Provider 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to execute this plan task-by-task. 本阶段实现默认可运行 WASM provider，但公共 contract 不绑定具体 engine。

## Goal

在 Runtime Host 执行面实现默认 in-process WASM runtime provider，使通过 admission 的 WASM application 能真实 compile、instantiate、invoke lifecycle/exported functions，并通过 provider-neutral host bridge 访问 Macaca service portal。默认实现可以选择 Wasmtime，但 Wasmtime 类型、配置和错误不得泄漏到公共 ABI/SDK/Application Framework。

## Scope

本阶段覆盖：

- default runtime provider module。
- component compile / instantiate / invoke 的最小闭环。
- compiled artifact cache strategy。
- sanitized trap/error mapping。
- runtime diagnostics 与 trace/log。
- 与 unavailable provider 共存的 provider registry。

本阶段不覆盖：

- hardened out-of-process provider。
- 完整 WASI policy。
- 全部 host imports。
- 生产级 Store certification 全量套件。

## Required Governance Inputs

- `2026-05-12-wasm-runtime-provider-contract-plan.md`
- `2026-05-12-wasm-package-admission-abi-negotiation-plan.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`

## Architecture Decision

采用 Abstract Factory + Strategy + Adapter + Observer：

- Abstract Factory：provider factory 创建 default provider、engine context、instance session。
- Strategy：compile cache、engine config、instantiation policy、error mapping 可替换。
- Adapter：具体 engine adapter 把 engine-specific API 转成 Macaca runtime provider contract。
- Observer：compile、cache hit/miss、instantiate、invoke、trap、shutdown 都写入 sanitized trace/log。
- Null Object：default provider 不可用时回退 unavailable provider，不 silently succeed。

## Proposed OpenSpec Change

建议 change id：

- `add-wasm-default-in-process-runtime-provider`

建议 specs：

- `wasm-default-runtime-provider`
- `wasm-runtime-error-taxonomy`
- `wasm-compiled-artifact-cache`

提案必须声明：

- default provider 是 optional execution provider，不是 Kernel dependency。
- 具体 engine dependency 只能出现在 provider implementation module。
- 所有 engine errors 必须映射为 provider-neutral error taxonomy。
- compiled cache key 必须包含 artifact digest、ABI version、engine capability、policy profile。

## Implementation Slices

### Slice 3.1：Impact 与 dependency audit

- [ ] 检查 workspace dependency policy，确认新增 engine dependency 的 crate 边界。
- [ ] 对 runtime host provider registry、application host factory、WASM host symbols 运行 GitNexus impact。
- [ ] 如果 impact HIGH/CRITICAL，先向用户报告 blast radius。

### Slice 3.2：OpenSpec default provider

- [ ] 定义默认 provider availability、compile、instantiate、invoke、shutdown 行为。
- [ ] 定义 error taxonomy：runtime unavailable、compile failed、ABI mismatch、trap、timeout、policy denied、resource exhausted。
- [ ] 定义 compiled cache memento，不存 raw bytes。

### Slice 3.3：Provider module 拆分

- [ ] 新增 provider module：descriptor、factory、engine_adapter、compile_cache、instance、errors、diagnostics。
- [ ] 每个 Rust 文件低于 500 行。
- [ ] 每个 public type/function 添加详细英文注释，解释边界和不变量。

### Slice 3.4：Compile / instantiate / invoke

- [ ] 从 artifact reference 读取受控 bytes，禁止日志记录 raw bytes。
- [ ] 编译 component/module 并记录 cache hit/miss。
- [ ] 创建 execution session，绑定 trace/resource/policy context。
- [ ] 调用最小 lifecycle export，返回 provider-neutral result。

### Slice 3.5：Error mapping 与 logs

- [ ] engine-specific trap/error 转换为 `WasmRuntimeErrorKind`。
- [ ] logs 只包含 artifact id/hash prefix、application id、ability id、trace id、reason code。
- [ ] raw guest payload、raw stdout/stderr、raw memory dump 禁止进入日志。

### Slice 3.6：测试

- [ ] unavailable provider 与 default provider registry 测试。
- [ ] compile success/failure 测试。
- [ ] invoke trap sanitized diagnostics 测试。
- [ ] cache key deterministic 测试。

## Validation

- `cargo check -p macaca-runtime-host`
- `cargo test -p macaca-runtime-host wasm_default_runtime`
- `cargo test -p macaca-integration-tests application_platform_contracts`
- `openspec validate add-wasm-default-in-process-runtime-provider --strict`
- `npx gitnexus detect-changes -r agent`

## Risks

- 风险：引入 engine dependency 导致编译体积或平台问题。缓解：dependency 只在 optional provider module，feature-gated 或 provider-gated。
- 风险：provider module 巨型化。缓解：compile、instance、cache、diagnostics、errors 分文件。
- 风险：error 泄露 raw payload。缓解：统一 sanitized diagnostics mapper。
