# Change: 增加 Memory Governance 与 Knowledge Layer

## Why

长期运行的 agent OS 不能只依赖原始对话片段和向量检索。原始记忆会变旧、冲突、污染或缺少来源，自动捕获如果直接写入长期记忆会降低 recall 质量。Macaca 需要候选层、晋升策略、审计日志、删除/tombstone、PII propagation，以及将原始记忆编译为 claim/evidence/decision/constraint 的知识层。

本变更建立 governance 与 knowledge layer，使记忆系统可长期自治、可审计、可纠错，并为上下文工程提供高质量 digest。

## What Changes

- 在 `macaca-memory` 单 crate 内增加 `governance/` 和 `artifacts/` 模块。
- 定义 memory candidate、promotion policy、audit event、tombstone、deletion propagation。
- 定义 knowledge compiler capability。
- 定义 claim/evidence/freshness/conflict/supersedes 数据模型。
- 支持 agent private candidate 与 session shared candidate。
- 支持 compiled digest、public artifacts、wiki/report 输出。
- 支持治理事件进入 trace/report，不默认泄露完整敏感内容。

## Impact

- Affected specs: `macaca-memory-governance`
- Affected code:
  - `macaca/crates/macaca-memory/src/governance/`
  - `macaca/crates/macaca-memory/src/artifacts/`
  - `macaca/crates/macaca-memory/src/core/`
  - 可选 `macaca-context` digest source 接入
- Compatibility:
  - 默认自动捕获可以保守启用或配置关闭。
  - 用户显式“记住”仍可直接写入高置信记忆。
  - 不新增额外 crate。
