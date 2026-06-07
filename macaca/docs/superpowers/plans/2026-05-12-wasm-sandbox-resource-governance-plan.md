# WASM 阶段 4：Sandbox 与 Resource Governance 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to execute this plan task-by-task. 本阶段把 WASM execution 从“能跑”提升到“可安全长期运行”。

## Goal

为 WASM application 建立工业级沙箱和资源治理：memory/table limits、fuel/epoch interruption、wall-clock timeout、host import timeout、payload size、instance concurrency、quota、WASI deny-by-default、capability-scoped preopen、no raw env/fs/network by default。

## Scope

本阶段覆盖：

- `WasmResourcePolicy` 与 `WasmSandboxPolicy`。
- resource envelope 与 quota admission。
- runtime enforcement strategy。
- WASI policy model。
- resource exhaustion diagnostics。
- trace/log/audit of key enforcement decisions。

本阶段不覆盖：

- 全量 host import service portal。
- out-of-process cgroup/container enforcement。
- 真实网络/文件 portal provider。

## Required Governance Inputs

- `2026-05-12-wasm-default-in-process-runtime-provider-plan.md`
- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/design_patterns.md`

## Architecture Decision

采用 Strategy + Decorator + Specification + Chain of Responsibility：

- Strategy：fuel/epoch、timeout、memory、WASI、payload、concurrency 都是可替换策略。
- Decorator：runtime session 外层叠加 trace、policy、resource guard、timeout guard。
- Specification：admission 阶段校验 manifest resource request 与 platform limits。
- Chain of Responsibility：每个 host call/execution request 依次通过 trace、policy、quota、payload、timeout guards。
- Observer：每个 deny/throttle/exhaustion 产生 sanitized audit event。

## Proposed OpenSpec Change

建议 change id：

- `add-wasm-sandbox-resource-governance`

建议 specs：

- `wasm-resource-policy`
- `wasm-sandbox-policy`
- `wasm-wasi-policy`
- `wasm-resource-audit`

提案必须声明：

- WASI 默认 deny。
- raw env、raw filesystem、raw network 默认不可用。
- 所有 resource limit 既要 admission 校验，也要 runtime enforcement。
- resource diagnostics 不允许包含 raw guest memory/payload。

## Implementation Slices

### Slice 4.1：Impact 与资源路径审计

- [ ] 审计 current runtime provider session、artifact loading、host invocation path。
- [ ] 对相关 symbols 运行 GitNexus impact。
- [ ] 明确哪些 limits 属于 Application Framework admission，哪些属于 Runtime Host enforcement。

### Slice 4.2：OpenSpec sandbox/governance

- [ ] 定义 resource policy DTO。
- [ ] 定义 sandbox policy DTO。
- [ ] 定义 WASI policy 和 capability preopen 规则。
- [ ] 定义 resource audit event 和 failure reason code。

### Slice 4.3：Resource policy model

- [ ] 增加 memory/table/fuel/epoch/time/payload/concurrency limits。
- [ ] 增加 app/session/ability scoped quota key。
- [ ] 增加 deterministic policy merge：platform default、deployment profile、manifest request、policy override。
- [ ] 新增 Rust 代码写详细英文注释。

### Slice 4.4：Runtime enforcement

- [ ] 在 compile/instantiate/invoke/session path 安装 resource guards。
- [ ] 实现 timeout 和 cancellation reason。
- [ ] 实现 payload bound check。
- [ ] 实现 concurrency admission。

### Slice 4.5：WASI deny-by-default

- [ ] 默认禁用 raw env/fs/network。
- [ ] 只允许 manifest 声明且 policy 批准的 virtual/preopen resource。
- [ ] logs 只记录 resource label 和 reason code，不记录 path secret 或 raw env。

### Slice 4.6：测试

- [ ] memory/fuel/timeout/payload limit 测试。
- [ ] raw env/fs/network denied 测试。
- [ ] policy merge deterministic 测试。
- [ ] resource audit sanitized 测试。

## Validation

- `cargo test -p macaca-app wasm_resource_admission`
- `cargo test -p macaca-runtime-host wasm_sandbox`
- `cargo test -p macaca-integration-tests route_c_dependency_boundaries`
- `openspec validate add-wasm-sandbox-resource-governance --strict`
- `npx gitnexus detect-changes -r agent`

## Risks

- 风险：只做 admission 不做 runtime enforcement。缓解：所有 policy 都必须有 runtime guard 测试。
- 风险：WASI policy 暴露过宽。缓解：deny-by-default，capability-scoped preopen。
- 风险：资源限制影响正常应用。缓解：policy profile 可配置，但默认保守。
