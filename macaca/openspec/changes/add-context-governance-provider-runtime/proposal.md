# Change: 增加 Context Governance 与 Provider Runtime

## Why

当 profile、active memory、skills、MCP、tools、trace、knowledge artifacts 都进入上下文工程后，Macaca 需要统一治理和 provider runtime：注册、配置、启用、超时、失败隔离、预算、redaction、trust boundary、审计和用户替换能力。如果缺少这一层，每个 provider 都会各自处理安全和失败逻辑，导致系统之间紧耦合且难以替换。

本提案建立 provider runtime 与治理层，使 Macaca 的上下文工程能像基础设施一样被组合、替换和审计。

## What Changes

- 新增 context provider registry/runtime，用于按配置创建、排序、启用和调用 providers。
- 定义 provider capability metadata、health、timeout、fallback 和 diagnostics。
- 定义统一治理策略：budget、redaction、trust promotion、source allow/deny、sensitive content handling。
- 支持用户替换 provider、composer、policy 或整套 context manager。
- 外部 provider 输出必须经过 Anti-Corruption Layer 校验，不能绕过 Macaca governance。
- runtime/framework 只依赖 `ContextFacade`，不直接依赖 provider runtime 内部。
- governance/report 覆盖 profile、memory、skills、MCP、tools、trace、knowledge artifacts。

## Impact

- Affected specs: `context-governance-runtime`
- Affected code:
  - `macaca/crates/macaca-context`
  - runtime/framework context integration
  - config/manifest provider selection
  - web/API context diagnostics
- Dependencies:
  - 依赖 `add-context-composer-foundation`。
  - 推荐在 profile、memory、skills/MCP providers 初步可用后实施。
- Compatibility:
  - 默认 provider runtime 使用内置 providers 和 legacy fallback。
  - 禁止 app/workflow/provider/business name 硬编码选择逻辑。
