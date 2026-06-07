# Tasks

## 1. 审计与影响分析

- [x] 1.1 阅读 context facade/composer/provider contract。
- [x] 1.2 阅读 config/manifest 中可用于 provider 选择的现有结构。
- [x] 1.3 阅读 context report/API/web diagnostics 路径。
- [ ] 1.4 对计划修改符号运行 GitNexus upstream impact analysis。

## 2. Provider Runtime

- [x] 2.1 定义 `ContextProviderRegistry`。
- [x] 2.2 定义 `ContextProviderFactory` 或 provider family factory。
- [ ] 2.3 定义 provider metadata、capability、health、version、policy hash。
- [ ] 2.4 实现按配置创建 provider set。
- [x] 2.5 禁止通过 app/workflow/business 名称硬编码 provider 选择。

## 3. Governance

- [x] 3.1 定义 budget governance strategy。
- [x] 3.2 定义 redaction strategy。
- [ ] 3.3 定义 trust classification/promotion policy。
- [x] 3.4 定义 source allow/deny policy。
- [x] 3.5 定义 timeout/fallback policy。
- [x] 3.6 将 governance 作为 provider decorator 或 composer preflight。

## 4. Runtime Integration

- [ ] 4.1 将内置 providers 注册到 runtime。
- [x] 4.2 runtime/framework 只调用 `ContextFacade`。
- [x] 4.3 将 provider diagnostics 合并到 `ContextReport`。
- [ ] 4.4 增加 diagnostics API 或 facade 方法读取 provider runtime 状态。

## 5. External/Custom Boundary

- [x] 5.1 定义自定义 provider in-process trait 接入方式。
- [ ] 5.2 定义外部 provider 输出校验模型，但不冻结远程协议。
- [x] 5.3 增加 schema/size/trust/source 校验失败诊断。

## 6. Tests

- [x] 6.1 单测 registry/factory 创建 provider set。
- [x] 6.2 单测 provider timeout/fallback。
- [x] 6.3 单测 redaction 和 trust policy。
- [x] 6.4 单测 provider failure 不阻塞模型调用。
- [x] 6.5 单测 report 包含 provider version/policy hash。

## 7. Verification

- [x] 7.1 运行 `openspec validate add-context-governance-provider-runtime --strict`。
- [x] 7.2 运行 `cargo test -p macaca-context` 及相关集成测试。
- [ ] 7.3 运行前后端 diagnostics smoke test。
- [ ] 7.4 运行 `gitnexus_detect_changes()`。
