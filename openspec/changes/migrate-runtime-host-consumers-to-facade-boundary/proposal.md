# Change: Migrate upper consumers to runtime-host facade boundary

## Why

`macaca-runtime-host` 已经完成基于设计模式的宿主层重构，提供了 `McpRuntimeFacade`、`McpServerFactory`、`RuntimeEnvBuilder` 等稳定边界。但上层消费代码，尤其是 `macaca-web`，仍残留对 `crate::mcp_runtime` 薄壳和 `mcp_runtime` 模块级 helper 的依赖，导致消费方边界没有真正收敛到新的宿主层抽象。

此外，仓库规则要求上层代码迁移离开被标记为 deprecated 的路径；即便当前 `macaca-web` 已很少直接调用显式 `#[deprecated]` API，仍需要把消费方从兼容薄壳、旧导入路径和不稳定 helper 依赖迁移到本轮 runtime-host 重构产出的主边界，避免后续继续沿旧路径扩散。

## What Changes

- 为 `macaca-runtime-host` 明确暴露上层 consumer 需要的稳定类型与 helper 边界。
- 迁移 `macaca-web` 的 MCP consumer 导入，从 `crate::mcp_runtime::*` 薄壳切换到 `macaca_runtime_host::*`。
- 优先迁移低风险入口：status route、app state、server bootstrap。
- 迁移 toolkit 组装路径和 skill MCP probe 路径，避免继续依赖 web 内部薄壳。
- 在确认无剩余上层调用后，删除 `macaca-web/src/mcp_runtime.rs` 薄壳模块。
- 保留 `macaca-runtime-host` 内部 deprecated 兼容路径，但仓库内上层 consumer 不再调用这些路径。

## Non-Goals

- 不改写 `macaca-runtime-host` 内部 manager / lease / factory 的核心行为。
- 不改变 MCP probe、tool registration、session cleanup 的语义或时机。
- 不删除 `macaca-runtime-host` 内部保留给外部兼容迁移检索的 deprecated API。
- 不引入应用专有逻辑、workflow 硬编码或新的外部依赖。

## Impact

- Affected specs: `macaca-runtime-host-consumers`
- Affected code:
  - `macaca/crates/macaca-runtime-host/src/lib.rs`
  - `macaca/crates/macaca-web/src/lib.rs`
  - `macaca/crates/macaca-web/src/state.rs`
  - `macaca/crates/macaca-web/src/routes.rs`
  - `macaca/crates/macaca-web/src/framework_toolkit.rs`
  - `macaca/crates/macaca-web/src/skill_mcp.rs`
  - `macaca/crates/macaca-web/src/mcp_runtime.rs`
- Expected risk: High
- GitNexus findings:
  - `build_toolkit` upstream risk = `HIGH`，直接影响 `framework_runner` 的 agent 构造链。
  - `probe_skill_mcp_servers` upstream risk = `HIGH`，影响 `get_app_skills` API 和对应测试。
  - `cleanup_session` upstream risk = `CRITICAL`，调用者位于 `post_chat_v2`；本 change 不改变其 cleanup 行为，只收敛导入和宿主边界。
  - `get_mcp_status`、`register_skill_backed_mcp_tools`、`post_chat_v2` 本身 upstream 风险为 `LOW`，适合作为先迁入口。
- Compatibility:
  - 运行时行为、MCP status schema、tool 名称、session cleanup 时机保持不变。
  - `macaca-runtime-host` deprecated compatibility path 继续保留，供仓库外或后续迁移检索。
