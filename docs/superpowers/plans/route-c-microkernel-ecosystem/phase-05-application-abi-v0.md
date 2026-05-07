# 阶段 5：Application ABI v0 细分实施计划

## 目标

把 Macaca Application 从 YAML-only 配置形态扩展为标准 Application ABI。阶段 5 不要求完整执行 WASM，但必须把生命周期、事件、能力调用、任务、trace、UI、storage、payment 等 ABI 边界设计并落地到 SDK/loader contract。

## 架构设计

Application ABI 是第三方软件运行在 Macaca OS 上的公共协议。Application 不能依赖内部 Rust crate，也不能绕过权限直接访问系统服务。WASM 是长期二进制基础，但 ABI 必须先于 runtime 实现稳定下来。

推荐设计模式：

- Facade：`ApplicationHost` 为 app 暴露受控系统能力。
- Adapter：YAML app、future WASM app、hybrid app 都适配到同一 ABI。
- Command：app 发起的 service call 表示为 host command。
- State：application lifecycle 显式建模。
- Memento：application state/checkpoint 可恢复。

## 涉及文件

- 新增：`macaca/crates/macaca-proto/src/application_abi.rs`
- 新增：`macaca/crates/macaca-app/src/abi.rs`
- 新增：`macaca/crates/macaca-app/src/host.rs`
- 新增：`macaca/crates/macaca-app/src/lifecycle.rs`
- 新增：`macaca/crates/macaca-sdk/src/application.rs`
- 新增：`macaca/crates/macaca-app/tests/application_abi.rs`
- 修改：`macaca/crates/macaca-framework`
- 修改：`macaca/crates/macaca-web`

## 抽象设计

ABI v0 必须定义 exports：

- `app:init`
- `app:start`
- `app:handle_event`
- `app:render`
- `app:pause`
- `app:resume`
- `app:shutdown`
- `app:upgrade`

ABI v0 必须定义 imports：

- `macaca:capability/request`
- `macaca:task/create_goal`
- `macaca:task/query`
- `macaca:trace/emit`
- `macaca:ui/render`
- `macaca:storage/get`
- `macaca:storage/set`
- `macaca:payment/create_intent`
- `macaca:service/call`

## 实施切片

### 切片 5.1：ABI proto 与文档

定义 ABI 类型、lifecycle state、host call 类型和错误模型。

验证：

- ABI 类型 serde roundtrip。
- lifecycle state transition 测试。

### 切片 5.2：ApplicationHost facade

新增 host facade，先通过现有 app/runtime 能力实现 task、trace、storage 的最小真实路径。

验证：

- app host 创建 goal 会进入现有 task path。
- app host emit trace 会进入 EventLog/RunTrace。

### 切片 5.3：YAML app ABI adapter

当前 YAML application 通过 adapter 被看作 ABI application。

验证：

- 现有 app 启动路径仍可用。
- adapter 生成 lifecycle events。

### 切片 5.4：WASM loader stub

WASM loader stub 只解析 manifest 与 ABI declaration，不执行 WASM。遇到执行请求返回结构化 `RuntimeUnavailable`。

验证：

- WASM package metadata 可以加载。
- 执行时明确失败，不 panic。

## 里程碑

- M5.1：ABI v0 类型和文档完成。
- M5.2：ApplicationHost 能调用 task/trace。
- M5.3：YAML app 通过 ABI adapter 运行。
- M5.4：WASM package 可加载 metadata。

## 禁止事项

- 禁止在阶段 5 实现半成品 WASM 执行器冒充完整 runtime。
- 禁止 Application 直接拿 `Arc<AppState>`。
- 禁止 app host 绕过 trace/policy。
- 禁止破坏现有 YAML app。

## 验收命令

```bash
cargo test -p macaca-app application_abi
cargo test -p macaca-sdk application
cargo check -p macaca-web
```

