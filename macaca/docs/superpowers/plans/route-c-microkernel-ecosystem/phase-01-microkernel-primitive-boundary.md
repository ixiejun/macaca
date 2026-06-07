# 阶段 1：微内核原语边界细分实施计划

## 目标

把 Macaca kernel 收敛为最小稳定原语集合。阶段 1 只定义和接入 additive contracts，不迁移所有上层逻辑，不实现 Store/WASM/Web3/GenUI。

## 架构设计

核心思路是把 kernel 从“很多系统能力的实现容器”变成“系统不变量的门面”。Kernel 不直接实现具体 LLM、driver、gateway、skill、memory，而是注册、发现、调度、授权、观测这些能力。

推荐设计模式：

- Facade：`KernelFacade` 暴露稳定入口，隐藏内部 executor/registry/scheduler。
- Registry：注册 system service、capability、resource scope。
- Strategy：scheduler、policy、resource allocation 后续可替换。
- Observer：trace/audit bus 作为所有 kernel primitive 的事件出口。
- State：session/task/agent 状态必须显式状态机化。

## 涉及文件

- 修改：`macaca/crates/macaca-proto/src/lib.rs`
- 新增：`macaca/crates/macaca-proto/src/kernel.rs`
- 修改：`macaca/crates/macaca-kernel/src/lib.rs`
- 新增：`macaca/crates/macaca-kernel/src/facade.rs`
- 新增：`macaca/crates/macaca-kernel/src/capability_registry.rs`
- 新增：`macaca/crates/macaca-kernel/src/service_registry.rs`
- 新增：`macaca/crates/macaca-kernel/src/policy.rs`
- 新增：`macaca/crates/macaca-kernel/src/resource.rs`
- 修改：`macaca/crates/macaca-sdk/src/lib.rs`
- 新增测试：`macaca/crates/macaca-kernel/tests/kernel_primitives.rs`

## 抽象设计

必须定义的基础类型：

- `KernelServiceId`
- `CapabilityId`
- `CapabilityDescriptor`
- `ServiceScope`
- `TraceContext`
- `PolicyDecision`
- `PolicyRequest`
- `ResourceScope`
- `KernelPrimitiveError`

必须定义的基础 trait：

- `CapabilityRegistry`
- `SystemServiceRegistry`
- `PolicyEngine`
- `TraceEventBus`
- `ResourceManager`
- `KernelFacade`

## 实施切片

### 切片 1.1：proto 层新增 kernel primitive 类型

在 `macaca-proto` 中新增纯数据类型，不依赖上层 crate。

验证：

- `cargo test -p macaca-proto`
- 所有类型可 serde 序列化/反序列化。
- 类型不引用 `macaca-web`、`macaca-app`、`macaca-framework`。

### 切片 1.2：kernel facade skeleton

在 `macaca-kernel` 中新增 facade，但先包装现有 registry/scheduler/event bus，不改变行为。

验证：

- 现有 kernel tests 通过。
- 新 facade 能注册 capability descriptor。
- 新 facade 能查询 capability descriptor。

### 切片 1.3：policy facade skeleton

新增 `PolicyEngine` trait 和默认 allow policy。默认策略只用于兼容，不代表长期安全策略。

验证：

- 默认 policy 返回结构化 `PolicyDecision::Allow`。
- deny path 有测试。

### 切片 1.4：resource manager skeleton

新增 resource scope 抽象，先只支持 workspace/browser/driver_process/network/storage 等枚举，不接管现有资源。

验证：

- 同一 resource scope 可被注册。
- 重复注册返回结构化错误。

### 切片 1.5：deprecated direct internals

对不应继续被上层长期直接调用的 kernel internals 标注 deprecated，并提供 facade 替代入口。

验证：

- `cargo check` 不引入新错误。
- deprecated 文案明确指向替代入口。

## 里程碑

- M1.1：`macaca-proto` 拥有 kernel primitive types。
- M1.2：`macaca-kernel` 拥有 facade/registry/policy/resource skeleton。
- M1.3：`macaca-sdk` 能通过 facade 查询 kernel primitive。
- M1.4：现有 Web 和 application 流程不变。

## 禁止事项

- 禁止在 kernel 中加入具体 Store、Web3、EVM、driver provider。
- 禁止引入 application-specific 名称。
- 禁止把所有现有 `macaca-web` orchestration 迁入 kernel。
- 禁止为了通过测试写空实现且无结构化错误。

## 验收命令

```bash
cargo test -p macaca-proto
cargo test -p macaca-kernel
cargo check -p macaca-web
rg -n "FULLSTACK|NEWSROOM|claude|opencode|discord|telegram" macaca/crates/macaca-kernel/src macaca/crates/macaca-proto/src
```

