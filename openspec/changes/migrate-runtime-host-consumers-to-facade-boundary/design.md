# Design

## Context

`macaca-runtime-host` 已完成 facade / bridge / lease / factory 渐进式重构，但 `macaca-web` 仍保留一个 [mcp_runtime.rs](file:///Users/quantum/Code/dev/agent/macaca/crates/macaca-web/src/mcp_runtime.rs) 薄壳模块，通过 `pub use macaca_runtime_host::mcp_runtime::*;` 把 runtime-host 内部模块重新暴露回 web crate。这样虽然短期兼容，但会让上层 consumer 继续沿旧边界编程，削弱新宿主层抽象的价值。

当前消费面主要集中在：

- [state.rs](file:///Users/quantum/Code/dev/agent/macaca/crates/macaca-web/src/state.rs)
- [routes.rs](file:///Users/quantum/Code/dev/agent/macaca/crates/macaca-web/src/routes.rs)
- [framework_toolkit.rs](file:///Users/quantum/Code/dev/agent/macaca/crates/macaca-web/src/framework_toolkit.rs)
- [skill_mcp.rs](file:///Users/quantum/Code/dev/agent/macaca/crates/macaca-web/src/skill_mcp.rs)

其中 `skill_mcp.rs` 是最明显的半迁移状态：definition 构建已使用 `McpServerFactory`，但 context、policy、status 和 probe helper 仍从 `crate::mcp_runtime` 或 runtime-host 内部模块取得。

## Goals

- 让仓库内上层 consumer 直接依赖 `macaca-runtime-host` 的稳定边界。
- 清理 `macaca-web` 对 `crate::mcp_runtime` 薄壳的依赖。
- 删除 `macaca-web/src/mcp_runtime.rs`，防止继续扩散旧导入路径。
- 保持行为兼容，不改 MCP 注册、probe、cleanup 语义。
- 对高风险 symbol 采用小切片迁移。

## Non-Goals

- 不改 runtime-host 内部生命周期实现。
- 不重构 `chat_orchestrator` 的 cleanup 控制流。
- 不删除 runtime-host 内部 deprecated API。
- 不把上层 consumer 迁移扩展到与 MCP 无关的模块。

## Pattern Mapping

- Facade: 上层统一依赖 `McpRuntimeFacade` 及 runtime-host crate 根导出。
- Adapter: 删除 web 薄壳前，先把 consumer 导入改成 runtime-host 稳定导出。
- Strangler Fig: 先迁低风险 consumer，再迁高风险 helper，最后移除薄壳模块。

## Decisions

- 在 [lib.rs](file:///Users/quantum/Code/dev/agent/macaca/crates/macaca-runtime-host/src/lib.rs) 为上层 consumer 补充稳定 re-export，避免 `macaca-web` 再直接引用 `mcp_runtime` 内部模块细节。
- `routes.rs`、`state.rs`、`lib.rs` 先迁到 crate 根导出，因为 blast radius 低。
- `framework_toolkit.rs` 和 `skill_mcp.rs` 单独迁移，因为它们分别挂在 `build_toolkit` 和 `probe_skill_mcp_servers` 的高风险链路上。
- 当 `macaca-web` 无剩余 `crate::mcp_runtime::*` 调用后，删除 web 薄壳模块和对应 `pub mod`。

## Risks / Trade-offs

- 风险：为上层提供更多 crate 根 re-export 可能扩大 runtime-host 公共表面。
  - 缓解：只导出已经公开且被多个上层 consumer 真正需要的类型/函数，不新增业务语义。

- 风险：`build_toolkit` 迁移会波及 `framework_runner`。
  - 缓解：只改 import 和边界，不改 tool registration 顺序、close callback 或错误文案。

- 风险：`probe_skill_mcp_servers` 迁移会影响 `/api/apps/:id/skills`。
  - 缓解：保持返回 DTO 和状态映射不变，并保留现有测试覆盖。

- 风险：删除 web 薄壳后若仍有漏网调用会导致编译失败。
  - 缓解：在删除前先做全仓 grep，确认仅剩 runtime-host 自身内部 `crate::mcp_runtime` 引用。

## Migration Plan

1. 创建并验证 OpenSpec change。
2. 在 runtime-host 根模块补 consumer 需要的稳定导出。
3. 迁移 `macaca-web` 低风险入口：server bootstrap、state、routes。
4. 迁移 `framework_toolkit.rs` 到 runtime-host 直接导入边界。
5. 迁移 `skill_mcp.rs` 到 runtime-host 直接导入边界。
6. 全仓确认无 `macaca-web` 对 `crate::mcp_runtime::*` 的剩余调用。
7. 删除 `macaca-web/src/mcp_runtime.rs` 和 `pub mod mcp_runtime;`。
8. 运行测试、编译、OpenSpec 校验和 GitNexus 变更检测。
