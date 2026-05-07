# 阶段 3：IPC / Service Bus 细分实施计划

## 目标

让系统服务调用与具体 transport 解耦。第一版必须 local-first、typed-first，避免过早引入跨进程序列化成本，但抽象必须能扩展到 child process、MCP、HTTP、远程 A2A。

## 架构设计

`macaca-ipc` 应从普通内部通信 crate 升级为 Agent OS service call 平面。所有跨服务调用都通过 envelope 携带 identity、trace、permission、session/task context。

推荐设计模式：

- Bridge：service command 与 transport 分离。
- Command：`ServiceCommand` 表示一次调用。
- Proxy：远程服务通过 local proxy 暴露同一接口。
- Decorator：trace、permission、metering 包装 transport。
- Strategy：transport selection 可替换。

## 涉及文件

- 修改：`macaca/crates/macaca-ipc/src/lib.rs`
- 新增：`macaca/crates/macaca-ipc/src/service_bus.rs`
- 新增：`macaca/crates/macaca-ipc/src/envelope.rs`
- 新增：`macaca/crates/macaca-ipc/src/transport.rs`
- 新增：`macaca/crates/macaca-ipc/src/local.rs`
- 新增：`macaca/crates/macaca-ipc/tests/service_bus.rs`
- 修改：`macaca/crates/macaca-kernel/src/service_call.rs`
- 修改：`macaca/crates/macaca-runtime-host`

## 抽象设计

核心类型：

- `ServiceEnvelope`
- `ServiceCommand`
- `ServiceReply`
- `ServiceTransport`
- `TransportKind`
- `TransportError`
- `ServiceBus`

Envelope 必须包含：

- `trace_context`
- `source_identity`
- `target_service_id`
- `session_id` 可选但推荐
- `task_id` 可选
- `permission_scope`
- `deadline`
- `idempotency_key`

## 实施切片

### 切片 3.1：Envelope 与 typed local transport

新增 envelope 和 local transport。local transport 不做 JSON 序列化，保持 typed payload 或 enum payload。

验证：

- local command 能发送到 mock service。
- reply 能带回 trace id。
- deadline 到期返回 timeout error。

### 切片 3.2：Transport trait 与 bridge

定义 `ServiceTransport`，让 local transport 成为第一个实现。

验证：

- 同一个 command 可通过 trait object 调用。
- transport error 可转换为 kernel service error。

### 切片 3.3：Trace decorator

新增 transport decorator，负责在发送前校验 trace context，在返回后记录 call outcome。

验证：

- 缺少 trace context 被拒绝。
- 成功和失败都 emit trace。

### 切片 3.4：Future transport extension points

只定义扩展点，不实现 child process/MCP/HTTP。

验证：

- 编译期可以定义 mock remote transport。
- 不影响 local transport。

## 里程碑

- M3.1：local service bus 可用。
- M3.2：transport bridge 可替换。
- M3.3：trace decorator 强制执行。
- M3.4：kernel 能通过 service bus 调 mock service。

## 禁止事项

- 禁止为了“真实跨进程”一次性引入复杂 runtime。
- 禁止所有调用都强制 JSON 化。
- 禁止 service bus 绕过 policy/trace。
- 禁止将具体 MCP 或 HTTP 细节写入核心 bus。

## 验收命令

```bash
cargo test -p macaca-ipc
cargo test -p macaca-kernel service_call
cargo check --workspace
```

