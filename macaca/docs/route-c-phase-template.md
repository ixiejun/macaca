# Macaca OS 路线 C 阶段实施模板

## 1. 使用规则

后续路线 C 每个阶段都必须复制本模板，并在该阶段 OpenSpec 前补全。不得跳过 OpenSpec 直接改代码。

## 2. 阶段头部

```markdown
# 阶段 N：[阶段名] 实施计划

## 目标

[写清本阶段要真正实现什么，以及不实现什么。]

## 设计模式

[列出本阶段使用的设计模式，并说明为什么。]

## 涉及 crate / 文件

[列出新增、修改、测试文件。]
```

## 3. 必须执行流程

固定 marker：`Superpowers brainstorm`、`OpenSpec proposal/design/tasks/spec`、`GitNexus impact`、`additive-first`、`targeted tests`、`integration smoke`、`detect_changes`、`commit`。

### 3.1 Superpowers Brainstorm

必须回答：

- 当前问题是什么？
- 为什么必须在本阶段解决？
- 有哪些可选方案？
- 推荐方案为什么最适合？
- 风险和回滚是什么？

### 3.2 OpenSpec

必须创建：

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/<capability>/spec.md`

每个 requirement 必须有 `#### Scenario:`。

### 3.3 GitNexus

改代码前：

- 对将修改的 symbol 跑 impact。
- HIGH/CRITICAL 必须先向用户说明风险。

提交前：

- 跑 `gitnexus detect_changes`。

### 3.4 Additive-first 实施

顺序必须是：

1. 新增 contract。
2. 新增 adapter/facade。
3. 新增测试。
4. 迁移一个 consumer。
5. 迁移剩余 consumers。
6. 标记旧 direct path deprecated。
7. 跑回归。
8. 提交。

## 4. 每阶段必须写明的禁止事项

每个阶段都必须显式写出：

- 不允许实现的未来能力。
- 不允许破坏的现有链路。
- 不允许出现的硬编码。
- 不允许的 demo/toy shortcut。

## 5. 验收门禁

每个阶段至少包含：

- OpenSpec strict validation。
- Targeted unit/integration tests。
- 至少一个 Route C 回归矩阵场景。
- `cargo check` 或更严格命令。
- GitNexus detect_changes。

## 6. Commit 要求

每个阶段可以多个 commit，但每个 commit 必须：

- 只包含一个 public contract 或一个 migration slice。
- 能编译。
- 有对应验证命令。
