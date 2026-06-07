## 1. OpenSpec

- [x] 1.1 创建 proposal、design、tasks 和 delta spec
- [x] 1.2 运行 `openspec validate migrate-runtime-host-consumers-to-facade-boundary --strict`

## 2. Baseline And Impact

- [x] 2.1 盘点 `macaca-web` 中所有 `crate::mcp_runtime::*` 和 runtime-host 旧边界调用
- [x] 2.2 对拟修改 symbol 运行 GitNexus impact，并记录风险
- [x] 2.3 在实施前向用户报告 `HIGH/CRITICAL` 风险与缓解策略

## 3. Runtime-Host Consumer Boundary

- [x] 3.1 在 `macaca-runtime-host` crate 根导出上层 consumer 需要的稳定类型和 helper
- [x] 3.2 保持 runtime-host 内部 deprecated compatibility path 不变

## 4. Low-Risk Consumer Migration

- [x] 4.1 迁移 `macaca-web/src/lib.rs` 到 runtime-host 直接导入
- [x] 4.2 迁移 `macaca-web/src/state.rs` 到 runtime-host 直接导入
- [x] 4.3 迁移 `macaca-web/src/routes.rs` 到 runtime-host 直接导入

## 5. High-Risk Consumer Migration

- [x] 5.1 迁移 `macaca-web/src/framework_toolkit.rs` 到 runtime-host 直接导入
- [x] 5.2 迁移 `macaca-web/src/skill_mcp.rs` 到 runtime-host 直接导入
- [x] 5.3 确保 `probe_skill_mcp_servers`、`build_toolkit` 不再通过 web 薄壳访问 runtime-host
- [x] 5.4 不改变 `post_chat_v2` cleanup 行为，只保留 facade 调用边界

## 6. Thin Shell Removal

- [x] 6.1 确认 `macaca-web` 无剩余 `crate::mcp_runtime::*` 调用
- [x] 6.2 删除 `macaca-web/src/mcp_runtime.rs`
- [x] 6.3 删除 `macaca-web/src/lib.rs` 中的 `pub mod mcp_runtime;`

## 7. Verification

- [x] 7.1 运行 `cargo test -p macaca-runtime-host`
- [x] 7.2 运行 `cargo check -p macaca-web`
- [x] 7.3 运行 `cargo test -p macaca-web skill_mcp -- --nocapture`
- [x] 7.4 grep 确认 `macaca-web` 不再引用 `crate::mcp_runtime::*`
- [x] 7.5 运行 `openspec validate migrate-runtime-host-consumers-to-facade-boundary --strict`
