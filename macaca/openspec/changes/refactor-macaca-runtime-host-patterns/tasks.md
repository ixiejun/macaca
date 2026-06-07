## 1. Preparation

- [x] 1.1 阅读 `macaca/crates/macaca-runtime-host/src/**`，确认当前 public API、测试覆盖和消费入口
- [x] 1.2 审计 `macaca-web` 等消费方对 `McpRuntimeManager`、definition helper、cleanup helper 的直接依赖点
- [x] 1.3 对将修改的关键 symbol 逐一执行 GitNexus impact analysis，并记录风险等级
- [x] 1.4 锁定现有行为测试：status probe、tool registration、runtime key cleanup、compat policy、env bridge
- GitNexus 记录：
  - `cleanup_session` upstream risk = `CRITICAL`，direct caller = `post_chat_v2`，命中 5 条 `post_chat_v2` 相关 process。
  - `probe_statuses` upstream risk = `LOW`，direct caller = `get_mcp_status`。
  - `register_tools` upstream risk = `LOW`；`McpRuntimeManager` upstream risk = `LOW` / `impactedCount=0`。
  - 新增 facade / lease / factory / builder / transport symbol 暂未被当前索引命中，已按 legacy entry points 记录 blast radius。

## 2. Slice 1: McpRuntimeFacade

- [x] 2.1 新增 `McpRuntimeFacade`，覆盖 probe / register / register_definitions / cleanup 等宿主层主动作
- [x] 2.2 让 facade 委托现有 `McpRuntimeManager`，不改变返回结构和行为
- [x] 2.3 在 crate 内优先迁移内部调用点到 facade
- [x] 2.4 为 facade 增加聚焦测试，确认与现有 manager 行为一致

## 3. Slice 2: McpTransport Bridge

- [x] 3.1 提取 `McpTransport` bridge，封装 stdio / sse / streamable_http client 创建逻辑
- [x] 3.2 用 adapter 接住现有 `McpTransportConfig` 与 `client_from_transport(...)`
- [x] 3.3 保持 `McpServerDefinition` 序列化结构兼容
- [x] 3.4 验证 probe 和 register 路径在不同 transport 下行为不变

## 4. Slice 3: McpSessionLease

- [x] 4.1 引入 `McpSessionLease`，显式承载 runtime key 与 cleanup command
- [x] 4.2 将 runtime key 获取/释放流程改为通过 lease 表达
- [x] 4.3 让 close callback、session cleanup、app cleanup 统一经过 lease release
- [x] 4.4 补充 task complete / fail / timeout 的释放路径验证

## 5. Slice 4: McpServerFactory + RuntimeEnvBuilder

- [x] 5.1 引入 `RuntimeEnvBuilder`，收口 env 注入、placeholder 过滤与 env forwarding 组装
- [x] 5.2 引入 `McpServerFactory`，统一 definition 构建、compat policy、隔离参数与 transport 装配
- [x] 5.3 让 skill snapshot、YAML config、compat registry 的 definition 构建逐步委托到 factory
- [x] 5.4 验证 definition 输出、required bins、tool prefix、lifecycle、session mode 保持兼容

## 6. Slice 5: Deprecated Compatibility Layer

- [x] 6.1 识别所有应保留但不再推荐的新旧 public API 边界
- [x] 6.2 为旧 public API 添加 `#[deprecated(note = "...")]` 与迁移说明
- [x] 6.3 确保 deprecated 旧接口只做委托，不再承载新增逻辑
- [x] 6.4 在 crate 内和主要消费方禁止新增对 deprecated 接口的调用
- [x] 6.5 保留旧接口源码，不删除，便于后续迁移检索

## 7. Consumer Migration

- [x] 7.1 优先迁移 `macaca-web` 到 facade 优先路径
- [x] 7.2 仅在每个切片稳定后再迁下一批消费方
- [x] 7.3 如有必要，为旧接口调用点保留 TODO 或迁移注释，避免遗漏
- [x] 7.4 确认消费方迁移过程中没有引入 app-specific 硬编码

## 8. Verification

- [x] 8.1 运行 `cargo test -p macaca-runtime-host`
- [x] 8.2 运行 `cargo check -p macaca-runtime-host`
- [x] 8.3 至少运行 `cargo check -p macaca-web`
- [x] 8.4 对资源隔离场景补充回归验证，尤其是并发 stateful MCP session
- [x] 8.5 运行 `openspec validate refactor-macaca-runtime-host-patterns --strict`
- [x] 8.6 每个切片完成后真实更新 checklist，禁止预先勾选
