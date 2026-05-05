## 1. Preparation

- [x] 1.1 阅读 `macaca-memory` facade/router/provider 相关新旧代码。
- [x] 1.2 阅读配置解析路径和现有 default config。
- [x] 1.3 对计划修改的 facade/router/provider factory 符号运行 GitNexus upstream impact analysis。

## 2. Provider registry and profiles

- [x] 2.1 新增 `providers/mod.rs`。
- [x] 2.2 新增 `MemoryProviderRegistry`。
- [x] 2.3 新增 `MemoryProviderFactory`。
- [x] 2.4 新增 `MemoryProfileConfig`、component slot DTO。
- [x] 2.5 支持 agent/session 级别 override。

## 3. Builtin provider

- [x] 3.1 实现 builtin provider factory。
- [x] 3.2 将 existing managers 适配为 builtin capabilities。
- [x] 3.3 默认 profile 未配置时选择 builtin provider。

## 4. Remote provider

- [x] 4.1 定义 `macaca-memory-v1` request/response schema。
- [x] 4.2 实现 remote status/search/get/write/delete/events client。
- [x] 4.3 所有 remote request 携带 `MemoryScope`、trace id、timeout。
- [x] 4.4 实现 timeout、payload limit、secret redaction、diagnostics。

## 5. MCP provider

- [x] 5.1 定义 MCP memory provider config。
- [x] 5.2 将 search/get/write/delete 映射到 MCP tools。
- [x] 5.3 对 MCP 返回做 schema validation 和 trust boundary 标记。

## 6. Resilience

- [x] 6.1 增加 circuit breaker。
- [x] 6.2 增加 retry policy。
- [x] 6.3 provider failure 记录 diagnostics，不终止 agent run。
- [x] 6.4 provider status 汇总到 `MemoryStatusReport`。

## 7. Tests

- [x] 7.1 profile 选择测试。
- [x] 7.2 agent private provider override 测试。
- [x] 7.3 session shared provider override 测试。
- [x] 7.4 remote provider schema 测试。
- [x] 7.5 MCP adapter schema 测试。
- [x] 7.6 provider failure graceful degradation 测试。

## 8. Verification

- [x] 8.1 运行 `cargo fmt`。
- [x] 8.2 运行 `cargo test -p macaca-memory`。
- [x] 8.3 运行相关上层 cargo check。
- [x] 8.4 运行 `openspec validate add-memory-provider-runtime --strict`。
- [ ] 8.5 运行 `gitnexus_detect_changes()`。
