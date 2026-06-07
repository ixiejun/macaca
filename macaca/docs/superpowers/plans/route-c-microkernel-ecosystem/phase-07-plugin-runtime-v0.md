# 阶段 7：Plugin Runtime v0 细分实施计划

## 目标

建立第三方扩展 Macaca OS 系统能力的标准机制。Plugin 可以提供 gateway、driver、memory、context、skill、MCP、payment adapter 等能力，但必须经过 manifest、权限、生命周期、trace、健康检查和卸载清理。

## 架构设计

Plugin Runtime v0 不急于执行任意第三方二进制。第一步是把 plugin manifest、plugin-provided service、lifecycle、permission、trace 和 built-in adapter 建立起来。第三方 WASM/native plugin execution 需要后续专门阶段。

推荐设计模式：

- Abstract Factory：按 plugin runtime kind 创建 plugin host。
- Adapter：内置 gateway/driver/memory 等适配成 plugin-provided service。
- Composite：一个 plugin 可提供多个 service/capability。
- State：plugin lifecycle 状态机。
- Proxy：未来进程外 plugin 通过 proxy 接入。

## 涉及文件

- 新增：`macaca/crates/macaca-proto/src/plugin.rs`
- 新增：`macaca/crates/macaca-runtime-host/src/plugin.rs`
- 新增：`macaca/crates/macaca-kernel/src/plugin_registry.rs`
- 修改：`macaca/crates/macaca-gateway`
- 修改：`macaca/crates/macaca-driver`
- 修改：`macaca/crates/macaca-memory`
- 修改：`macaca/crates/macaca-skill`
- 新增测试：`macaca/crates/macaca-runtime-host/tests/plugin_runtime.rs`

## 抽象设计

Plugin manifest 必须包含：

- `plugin_id`
- `version`
- `developer_id`
- `runtime.kind`
- `provides.services`
- `provides.capabilities`
- `requires.services`
- `permissions`
- `resources`
- `entry`
- `signature`

Lifecycle：

```text
installed -> registered -> starting -> running -> stopping -> stopped -> uninstalled
```

## 实施切片

### 切片 7.1：plugin manifest schema

在 proto 定义 plugin manifest。

验证：

- gateway plugin、driver plugin、memory plugin fixture 都能解析。
- 缺 permissions 的 plugin 被 runtime guard 拒绝。

### 切片 7.2：plugin registry

kernel/runtime-host 增加 plugin registry，能注册 plugin-provided service descriptor。

验证：

- 一个 plugin 可以注册多个 service。
- 卸载 plugin 会移除其 service。

### 切片 7.3：built-in gateway as plugin model

把现有 gateway 能力用 adapter 建模为 plugin-provided gateway service，不改变运行路径。

验证：

- gateway descriptor 可查询。
- gateway 缺失时 base OS 不受影响。

### 切片 7.4：plugin lifecycle trace

安装、注册、启动、停止、卸载都必须 emit trace。

验证：

- 每个 lifecycle transition 都出现在 EventLog。
- 失败 transition 也记录错误。

## 里程碑

- M7.1：Plugin manifest v0 稳定。
- M7.2：Plugin registry 可用。
- M7.3：Gateway 可被建模为 plugin service。
- M7.4：Plugin lifecycle trace 完整。

## 禁止事项

- 禁止直接执行未签名第三方代码。
- 禁止 plugin 绕过 service registry。
- 禁止 plugin 提供能力但不声明权限。
- 禁止 plugin 卸载后残留 service。

## 验收命令

```bash
cargo test -p macaca-proto plugin
cargo test -p macaca-runtime-host plugin_runtime
cargo test -p macaca-kernel plugin_registry
```

