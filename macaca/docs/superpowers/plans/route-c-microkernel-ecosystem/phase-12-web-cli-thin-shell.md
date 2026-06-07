# 阶段 12：Web / CLI Thin Shell 迁移细分实施计划

## 目标

把 `macaca-web` 和 `macaca-cli` 从事实上的系统协调层迁移为 thin shell。Web/CLI 只负责入口适配、命令转换、SSE/GenUI 渲染、用户确认、包管理界面，不再拥有核心 session/task/resume/service/package 语义。

## 架构设计

Presentation 层必须通过 SDK/Application/Kernel facades 调用系统能力。`macaca-web` 不应该直接拼装 agent、task loop、service call、entitlement、plugin、Web3 等核心逻辑。Frontend 是 Macaca Shell，不是唯一应用 UI。

推荐设计模式：

- Facade：Web/CLI 统一通过 SDK facade 调用系统。
- Command：HTTP/CLI input 转换为 system command。
- Adapter：SSE、HTTP、CLI、Gateway 各自只是 adapter。
- Observer：Web 订阅 trace/event，不定义 trace 语义。
- Visitor：frontend 渲染 GenUI/trace/package metadata。

## 涉及文件

- 修改：`macaca/crates/macaca-web/src/chat_orchestrator.rs`
- 修改：`macaca/crates/macaca-web/src/loop_manager.rs`
- 修改：`macaca/crates/macaca-web/src/framework_runner.rs`
- 修改：`macaca/crates/macaca-web/src/routes.rs`
- 新增：`macaca/crates/macaca-web/src/shell.rs`
- 新增：`macaca/crates/macaca-sdk/src/system_facade.rs`
- 修改：`macaca/crates/macaca-cli`
- 修改：`frontend/app`
- 修改：`frontend/components`

## 抽象设计

Web Shell 职责：

- route request -> SDK command
- SSE subscribe -> trace stream
- GenUI render -> UI event
- permission approval -> policy decision
- payment approval -> payment decision
- package install UI -> store command

CLI Shell 职责：

- inspect services
- install package
- run app
- inspect session
- tail trace
- approve/deny pending decisions

## 实施切片

### 切片 12.1：SDK system facade

把 session、task、trace、package、service 查询能力先统一到 SDK facade。

验证：

- Web route 可以通过 facade 获取 task board。
- CLI 可以通过 facade 查询 service registry。

### 切片 12.2：Web route command adapter

把一个低风险 route 迁移为 command adapter，例如 task board/session events。

验证：

- API 响应 1:1 不变。
- frontend 不需要改调用。

### 切片 12.3：trace/SSE 订阅薄化

Web 只订阅 trace service，不再自行定义核心事件语义。

验证：

- 实时 trace 不重复。
- 刷新后历史 trace 完整。

### 切片 12.4：GenUI shell mount

Frontend 增加应用 UI mount 区域，chat/trace shell 保持默认。

验证：

- 无 GenUI app 默认显示 chat。
- 有 GenUI app 显示自定义 surface。

### 切片 12.5：CLI facade 迁移

CLI 命令改为调用 SDK facade，而不是直接依赖 web/internal state。

验证：

- `macaca app list`、`macaca session inspect`、`macaca trace tail` 等命令走 facade。

## 里程碑

- M12.1：SDK facade 可用。
- M12.2：至少一个 Web route 成功 thin shell 化。
- M12.3：trace/SSE 不再重复且历史一致。
- M12.4：GenUI shell mount 可用。
- M12.5：CLI 通过 facade 调用系统。

## 禁止事项

- 禁止一次性重写整个 `macaca-web`。
- 禁止 Web/CLI 重新定义 kernel/service/app contract。
- 禁止破坏现有 Web UI。
- 禁止 frontend 写死某个 application UI。

## 验收命令

```bash
cargo test -p macaca-sdk
cargo test -p macaca-web
cargo check -p macaca-cli
cd frontend && npm run lint && npx tsc --noEmit
```

