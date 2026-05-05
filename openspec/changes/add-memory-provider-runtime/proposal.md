# Change: 增加可插拔 Memory Provider Runtime

## Why

Macaca 需要让默认记忆系统可替换、可组合、可自由装配。小白用户应能通过 endpoint/API key/MCP server 替换默认记忆系统；高级用户应能为 agent private、session shared、embedding、vector backend、active recall、knowledge compiler 配置不同组件。

本变更建立 provider runtime、registry、profile 配置、远程协议和 MCP adapter 边界，使记忆系统不和任何具体供应商或业务 application 强耦合。

## What Changes

- 在 `macaca-memory` 单 crate 内新增 `providers/` 模块。
- 定义 `MemoryProviderRegistry`、`MemoryProviderFactory`、provider profile、component slots。
- 支持 agent private provider 与 session shared provider 独立配置。
- 支持 builtin、remote、MCP provider adapter。
- 定义 `macaca-memory-v1` 远程 HTTP 协议。
- 定义 provider status、diagnostics、timeout、circuit breaker、secret redaction、scope mapping 要求。
- 支持 provider tools 注册和 tool name conflict 处理。

## Impact

- Affected specs: `macaca-memory-provider-runtime`
- Affected code:
  - `macaca/crates/macaca-memory/src/providers/`
  - `macaca/crates/macaca-memory/src/core/provider.rs`
  - `macaca/crates/macaca-memory/src/core/status.rs`
  - 配置解析相关路径
- Compatibility:
  - 默认 provider 为 builtin。
  - 未配置 remote/MCP 时行为不变。
  - 不新增额外 crate。
