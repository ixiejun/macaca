## 1. Preparation

- [ ] 1.1 阅读 `macaca-memory` facade/router/provider 相关新旧代码。
- [ ] 1.2 阅读配置解析路径和现有 default config。
- [ ] 1.3 对计划修改的 facade/router/provider factory 符号运行 GitNexus upstream impact analysis。

## 2. Provider registry and profiles

- [ ] 2.1 新增 `providers/mod.rs`。
- [ ] 2.2 新增 `MemoryProviderRegistry`。
- [ ] 2.3 新增 `MemoryProviderFactory`。
- [ ] 2.4 新增 `MemoryProfileConfig`、component slot DTO。
- [ ] 2.5 支持 agent/session 级别 override。

## 3. Builtin provider

- [ ] 3.1 实现 builtin provider factory。
- [ ] 3.2 将 existing managers 适配为 builtin capabilities。
- [ ] 3.3 默认 profile 未配置时选择 builtin provider。

## 4. Remote provider

- [ ] 4.1 定义 `macaca-memory-v1` request/response schema。
- [ ] 4.2 实现 remote status/search/get/write/delete/events client。
- [ ] 4.3 所有 remote request 携带 `MemoryScope`、trace id、timeout。
- [ ] 4.4 实现 timeout、payload limit、secret redaction、diagnostics。

## 5. MCP provider

- [ ] 5.1 定义 MCP memory provider config。
- [ ] 5.2 将 search/get/write/delete 映射到 MCP tools。
- [ ] 5.3 对 MCP 返回做 schema validation 和 trust boundary 标记。

## 6. Resilience

- [ ] 6.1 增加 circuit breaker。
- [ ] 6.2 增加 retry policy。
- [ ] 6.3 provider failure 记录 diagnostics，不终止 agent run。
- [ ] 6.4 provider status 汇总到 `MemoryStatusReport`。

## 7. Tests

- [ ] 7.1 profile 选择测试。
- [ ] 7.2 agent private provider override 测试。
- [ ] 7.3 session shared provider override 测试。
- [ ] 7.4 remote provider schema 测试。
- [ ] 7.5 MCP adapter schema 测试。
- [ ] 7.6 provider failure graceful degradation 测试。

## 8. Verification

- [ ] 8.1 运行 `cargo fmt`。
- [ ] 8.2 运行 `cargo test -p macaca-memory`。
- [ ] 8.3 运行相关上层 cargo check。
- [ ] 8.4 运行 `openspec validate add-memory-provider-runtime --strict`。
- [ ] 8.5 运行 `gitnexus_detect_changes()`。
