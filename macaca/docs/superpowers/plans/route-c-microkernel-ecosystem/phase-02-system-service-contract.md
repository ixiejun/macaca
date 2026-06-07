# 阶段 2：系统服务 Contract 细分实施计划

## 目标

建立统一 `SystemService` contract，让 LLM、Memory、Task、Trace、Driver、Skill、MCP、Gateway、Store、Payment、Web3、EVM、UI 都能作为可注册、可调用、可观测、可替换的系统服务存在。

## 架构设计

阶段 2 的关键不是马上迁移所有服务，而是定义所有服务必须遵守的生命周期和调用契约。现有内置实现通过 adapter 接入，第三方服务后续通过 plugin 或 IPC 接入。

推荐设计模式：

- Adapter：把现有 direct implementation 包装为 `SystemService`。
- Abstract Factory：根据 manifest/service descriptor 创建服务实例。
- Command：一次 service call 表示为 `ServiceCommand`。
- Observer：每次 call 都 emit trace event。
- Chain of Responsibility：permission、budget、trace、metering middleware 按链路执行。

## 涉及文件

- 新增：`macaca/crates/macaca-proto/src/service.rs`
- 修改：`macaca/crates/macaca-proto/src/lib.rs`
- 新增：`macaca/crates/macaca-kernel/src/system_service.rs`
- 新增：`macaca/crates/macaca-kernel/src/service_lifecycle.rs`
- 新增：`macaca/crates/macaca-kernel/src/service_call.rs`
- 新增：`macaca/crates/macaca-kernel/tests/system_service_contract.rs`
- 修改：`macaca/crates/macaca-llm`
- 修改：`macaca/crates/macaca-memory`
- 修改：`macaca/crates/macaca-task`
- 修改：`macaca/crates/macaca-driver`
- 修改：`macaca/crates/macaca-skill`
- 修改：`macaca/crates/macaca-gateway`

## 抽象设计

`SystemService` 必须至少包含：

- `service_id`
- `service_type`
- `capabilities`
- `lifecycle_state`
- `health`
- `required_permissions`
- `supported_scopes`
- `trace_schema`
- `cleanup_policy`

Service lifecycle：

```text
install -> register -> authorize -> start -> call -> trace -> stop -> cleanup
```

Service type 必须是开放枚举或可扩展 string newtype，不能写死为封闭业务枚举。

## 实施切片

### 切片 2.1：proto service descriptors

定义 `ServiceDescriptor`、`ServiceType`、`ServiceCapability`、`ServiceLifecycleState`、`ServiceHealth`。

验证：

- serde roundtrip。
- service type 支持未来扩展，不需要改 kernel enum。

### 切片 2.2：kernel service trait

定义 `SystemService` trait、`ServiceCallContext`、`ServiceCallResult`、`ServiceError`。

验证：

- mock service 可注册、start、call、stop。
- failed service 返回结构化错误。

### 切片 2.3：内置服务 adapter 第一批

先为 LLM、Task、Trace 建立 adapter skeleton，不迁移核心调用。

验证：

- adapter 可以导出 descriptor。
- health check 可执行。
- 不影响现有业务链路。

### 切片 2.4：内置服务 adapter 第二批

为 Driver、Skill、Gateway、Memory 建立 adapter skeleton。

验证：

- 每个 adapter 都有 capability descriptor。
- driver/skill/gateway 名称不进入 kernel。

### 切片 2.5：service call trace middleware

每次 service call 必须携带 `TraceContext`，没有 trace context 返回错误。

验证：

- 无 trace context 的 mock call 被拒绝。
- 有 trace context 的 mock call 产生 trace event。

## 里程碑

- M2.1：服务 descriptor 类型稳定。
- M2.2：kernel 可注册 mock system service。
- M2.3：现有核心服务可被描述为 service。
- M2.4：service call trace 规则生效。

## 禁止事项

- 禁止迁移全部业务逻辑到新 service bus。
- 禁止封闭 service type 造成第三方无法扩展。
- 禁止无 trace 的 service call。
- 禁止在 service contract 中出现具体 app 或 provider 名。

## 验收命令

```bash
cargo test -p macaca-proto service
cargo test -p macaca-kernel system_service
cargo check --workspace
rg -n "FULLSTACK|NEWSROOM|discord|telegram|claude|opencode" macaca/crates/macaca-kernel/src/system_service.rs macaca/crates/macaca-proto/src/service.rs
```

