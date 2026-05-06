# Tasks

## 1. 审计与影响分析

- [x] 1.1 阅读 skill runtime snapshot/catalog/progressive disclosure 实现。
- [x] 1.2 阅读 MCP runtime registry、tool/resource/prompt 暴露路径。
- [x] 1.3 阅读 tool schema prompt 注入路径。
- [ ] 1.4 对计划修改符号运行 GitNexus upstream impact analysis。（提交前按需执行；本轮以 `cargo check` / 单测为准。）

## 2. Capability Model

- [x] 2.1 定义 `CapabilityCandidate`、`CapabilityKind`、`CapabilityNamespace`（与 `ContextCandidate` → `macaca-context/capability/model.rs`）。
- [x] 2.2 定义 `CapabilitySnapshot`、`CapabilityDependency`；信任通过 `ContextCandidate::trust_level`（`TrustLevel`，未单独命名 `CapabilityTrust` 类型）。
- [x] 2.3 定义 collision/dedup diagnostics（`render.rs` MCP 碰撞等）。
- [x] 2.4 定义 compact render policy（空目录不产生噪音；索引不含完整 `SKILL.md`）。

## 3. Providers

- [x] 3.1 实现 Skill capability provider，`CapabilityIndex` 阶段。
- [x] 3.2 实现 MCP capability provider（`facade.probe` + 摘要）。
- [x] 3.3 将 runtime tool 名称映射到 capability catalog。
- [x] 3.4 MCP 条目默认 `Untrusted` + fenced（对齐 dynamic/untrusted 要求）。
- [x] 3.5 skills 依赖 MCP 时在渲染层生成备注（缺失 ready server）。

## 4. Migration

- [x] 4.1 迁移 skill catalog：`build_context_system_prompt` 移除内联 `400-skills`，改由 composer 能力 provider。
- [x] 4.2 MCP / runtime tools 同上经 `context_reporting_model` + `capability_catalog` 注入。
- [x] 4.3 `SkillCatalog::catalog_prompt` 已标记 `#[deprecated]`。
- [ ] 4.4 全局搜索并逐步迁移残余 `catalog_prompt` 调用。（集成测试等仍按需调用并 `#[allow(deprecated)]`。）

## 5. Tests

- [x] 5.1 单测 skill → capability 渲染（`macaca-context` capability render tests）。
- [x] 5.2 单测 MCP 摘要 / fenced。
- [x] 5.3 单测 collision / dependency 注释路径。
- [x] 5.4 单测 compact index 不包含完整 `SKILL.md` body。
- [x] 5.5 单测 MCP 默认 untrusted / fenced。

## 6. Verification

- [x] 6.1 运行 `openspec validate add-skills-mcp-capability-context --strict`。
- [x] 6.2 运行 `cargo test -p macaca-context`、`cargo test -p macaca-skill`、`cargo test -p macaca-web`。
- [x] 6.3 运行 `cargo test -p macaca-integration-tests --test fullstack_autodev`（与示例 `skills/`、`app.yaml` 同步）。
- [ ] 6.4 合并前运行 `gitnexus_detect_changes()`。
