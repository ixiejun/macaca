# 阶段 0：基线与治理细分实施计划

## 目标

建立后续 13 个阶段都必须遵守的工程基线、架构治理规则和回归验证矩阵。这个阶段不改变业务行为，但必须让后续每一次微内核化、服务化、Application ABI、Store、Web3/EVM 迁移都有可验证的安全网。

## 架构设计

阶段 0 的核心抽象是“治理即基础设施”。不是写一份说明文档，而是把架构边界、回归用例、OpenSpec 模板、阶段验收门禁固化为可复用资产。

推荐设计模式：

- Template Method：定义每个阶段统一执行模板，避免不同阶段随意发挥。
- Specification：把“什么属于 kernel / service / plugin / optional module”写成可检查规则。
- Observer：将 trace、session replay、task lifecycle 作为所有阶段共同观察点。
- Memento：把回归 session、event log、task board 状态作为可恢复验收样本。

## 涉及文件

- 创建：`macaca/docs/agent-os-microkernel-boundaries.md`
- 创建：`macaca/docs/route-c-regression-matrix.md`
- 创建：`macaca/docs/route-c-phase-template.md`
- 创建：`macaca/docs/route-c-architecture-governance.md`
- 修改：`macaca/docs/SYSTEM_OVERVIEW.md`
- 修改：`macaca/docs/design-pattern-refactor-plans/refactor-order.md`
- 可能新增：`macaca/crates/macaca-integration-tests/tests/route_c_baseline.rs`

## 实施切片

### 切片 0.1：微内核边界治理文档

- 写清 kernel 只允许承载 identity、scheduler、service registry、IPC/service bus、policy、trace/audit bus、resource manager、session/task primitive、package runtime guard。
- 写清 LLM、Memory、Driver、Skill、MCP、Gateway、Store、Payment、Web3、EVM、GenUI 都必须是 system service 或 optional module。
- 写清 application-specific 逻辑禁止进入 kernel。
- 写清 `macaca-web` 和 `macaca-cli` 最终必须成为 thin shell。

验证：

- 文档中每个 crate 都能映射到目标层。
- 文档没有使用“暂时先放 kernel”这类模糊表述。

### 切片 0.2：回归矩阵

建立必须长期保留的回归场景：

- YAML application 加载。
- `/api/chat/v2` 创建 session。
- `/api/chat/v2` 恢复 session。
- goal -> planner -> task -> worker -> review -> coordinator resume。
- trace 实时推送。
- trace 历史恢复。
- task board session-scoped 查询。
- driver trace。
- skill/MCP smoke path。
- frontend/backend 重启后 session 可恢复。

验证：

- 每个场景必须有明确输入、预期输出、观测点和失败判定。
- 每个后续阶段必须引用至少一个回归场景。

### 切片 0.3：阶段实施模板

模板必须包含：

- Superpowers brainstorm。
- OpenSpec proposal/design/tasks/spec。
- GitNexus impact。
- additive-first contract。
- targeted tests。
- integration smoke。
- detect_changes。
- commit。

验证：

- 后续阶段 plan 可以直接套用该模板。
- 模板中明确要求“不允许一次性大改”。

### 切片 0.4：集成测试骨架

如果现有 integration tests 不足，新增 route C baseline 测试骨架。第一版可以只封装 smoke runner，不要求覆盖所有未来能力。

验证：

- `cargo test -p macaca-integration-tests` 能运行 baseline smoke。
- 失败时能显示具体阶段和场景。

## 里程碑

- M0.1：微内核边界文档完成。
- M0.2：回归矩阵完成。
- M0.3：阶段模板完成。
- M0.4：至少一个 baseline smoke 自动化可运行。

## 禁止事项

- 禁止在阶段 0 实现 Store、WASM、Web3、EVM。
- 禁止把治理文档写成泛泛原则，必须映射到当前 crate。
- 禁止用“demo smoke”替代真实现有链路。

## 验收命令

```bash
rg -n "application-specific|Web3|EVM|GenUI|service registry" macaca/docs/agent-os-microkernel-boundaries.md
rg -n "chat/v2|trace|task board|coordinator resume" macaca/docs/route-c-regression-matrix.md
cargo test -p macaca-integration-tests
```

