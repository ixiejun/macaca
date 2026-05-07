# 阶段 6：GenUI Runtime v0 细分实施计划

## 目标

让 Macaca application 拥有自主 UI 能力。GenUI 不是替代聊天界面的小组件，而是 Application Framework 的基础能力：应用可以定义符合自身业务、品牌和交互逻辑的界面，同时保留系统级 trace、权限、支付确认和安全边界。

## 架构设计

GenUI v0 采用受控声明式 UI schema，不开放任意远程 UI 代码执行。应用通过 ABI 输出 `UiIntent` 或 `UiComponentTree`，Web Shell 负责渲染，用户交互转为 `UiEvent` 回到 application/session。

推荐设计模式：

- Visitor：后端/前端遍历 UI component tree 做渲染、权限检查、trace 标注。
- Composite：UI component tree。
- Command：用户交互转为 `UiEventCommand`。
- Decorator：系统 trace overlay、permission prompt、payment approval 包装应用 UI。
- Strategy：renderer 可按 Web、CLI、未来 desktop/mobile 替换。

## 涉及文件

- 新增：`macaca/crates/macaca-proto/src/ui.rs`
- 新增：`macaca/crates/macaca-app/src/genui.rs`
- 新增：`macaca/crates/macaca-web/src/genui_routes.rs`
- 修改：`macaca/crates/macaca-web/src/routes.rs`
- 新增：`frontend/components/genui/GenUiRenderer.tsx`
- 新增：`frontend/lib/genui.ts`
- 新增：`frontend/components/genui/TraceOverlay.tsx`
- 新增测试：`frontend/components/genui/GenUiRenderer.test.tsx`

## 抽象设计

UI v0 基础类型：

- `UiIntent`
- `UiComponentTree`
- `UiComponent`
- `UiEvent`
- `UiAction`
- `UiBinding`
- `UiPermissionPrompt`
- `UiTraceMarker`

组件 v0 范围：

- text
- markdown
- form
- button
- table
- card
- list
- chart placeholder
- trace panel mount
- approval prompt

## 实施切片

### 切片 6.1：UI proto schema

定义 UI schema 和事件类型。

验证：

- serde roundtrip。
- unknown component 返回 unsupported component，不 panic。

### 切片 6.2：Application GenUI API

ApplicationHost 增加 render UI 能力，输出 `UiIntent`。

验证：

- fixture app 可以输出 UI intent。
- UI intent 携带 session/app trace context。

### 切片 6.3：Web Shell renderer boundary

新增 GenUI renderer，不改现有 chat/trace 默认界面。

验证：

- 无 custom UI 的 app 仍显示 chat shell。
- 有 custom UI 的 fixture app 能 mount GenUI surface。

### 切片 6.4：UI event 回流

用户点击按钮、提交表单会生成 `UiEvent`，通过 `/api` 回到 session/application。

验证：

- UI event 被 EventLog 记录。
- UI event 能触发 app handler。

## 里程碑

- M6.1：UI schema v0 可序列化。
- M6.2：ApplicationHost 可 emit UI intent。
- M6.3：Frontend 可渲染受控组件树。
- M6.4：UI event 可 trace 回流。

## 禁止事项

- 禁止开放任意远程 JS 执行。
- 禁止让 GenUI 替代现有 chat shell。
- 禁止 UI event 不带 trace。
- 禁止把应用 UI 逻辑写死在 frontend。

## 验收命令

```bash
cargo test -p macaca-proto ui
cargo test -p macaca-app genui
cd frontend && npm run lint && npx tsc --noEmit
```

