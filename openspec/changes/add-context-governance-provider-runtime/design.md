# Design: Context Governance 与 Provider Runtime

## Context

Macaca 需要同时支持内置 providers 和用户自定义上下文系统。开放能力会带来安全、性能、稳定性和可观测性问题，因此 provider runtime 必须成为治理边界，而不是简单的插件列表。

## Goals / Non-Goals

Goals:

- 通过 registry/factory 创建和管理 providers。
- 统一 timeout、fallback、health、diagnostics。
- 统一 budget、redaction、trust、allow/deny policy。
- 支持用户替换 provider family 或整套 context manager。
- 保证所有外部输出经过校验和治理。

Non-Goals:

- 不冻结远程 plugin/RPC/WASM 协议。
- 不实现 marketplace。
- 不把 provider runtime 暴露成 application-specific scripting 环境。
- 不让 web UI 直接调用 provider。

## Decisions

### Decision 1: Provider Runtime 使用 Registry + Abstract Factory

provider runtime 根据配置从 registry 中选择 provider factory。factory 创建 provider family，例如 profile provider family、memory provider family、capability provider family。

理由：支持用户替换，同时保持 core 不依赖具体 provider 类型。

### Decision 2: Governance 使用 Decorator + Strategy

所有 provider 输出都经过治理 decorator：

- budget guard
- redaction guard
- trust classifier
- source allow/deny
- timeout/fallback
- diagnostics collector

策略可替换，但默认策略保守。

### Decision 3: External provider 使用 Anti-Corruption Layer

未来外部 provider 或自定义 context manager 的输出必须被转换成 Macaca 内部 candidate/report 模型，并通过 schema、大小、trust、source、budget 校验。

### Decision 4: Runtime/framework 只看 Facade

provider runtime 是 composition root 细节。runtime/framework 不知道 provider 列表，不直接调用 memory/skill/MCP/profile providers。

## Risks / Trade-offs

- Risk: provider runtime 太复杂。Mitigation: 首版仅 in-process registry/factory，不做远程协议。
- Risk: provider 失败阻塞模型调用。Mitigation: per-provider timeout 和 fail-open/fail-closed policy。
- Risk: 用户 provider 绕过治理。Mitigation: facade 只接受经过 runtime 校验后的 candidates。
- Risk: 配置能力导致不可复现。Mitigation: provider set、version、policy hash 写入 report。

## Migration Plan

1. 实现 provider registry/runtime。
2. 将内置 profile/memory/capability providers 通过 registry 注册。
3. 增加 governance decorators。
4. 将 runtime/framework 绑定到 facade。
5. 增加 diagnostics API，支持 web 查看 provider 状态和 report。
