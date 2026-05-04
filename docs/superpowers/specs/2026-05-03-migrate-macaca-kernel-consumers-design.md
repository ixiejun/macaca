# 迁移 macaca-kernel 上层消费方 Brainstorm 设计记录

## 背景

本轮目标不是继续扩展 `macaca-kernel` 自身抽象，而是把上层代码迁移到上一轮基于设计模式重构后的 kernel primitives。

当前磁盘上的 `macaca-kernel` 重构已经引入：

- `KernelBuilder`：新的 kernel 构造入口，替代生产代码直接调用 `Kernel::new`。
- `SchedulerFactory` / `SchedulerKind`：scheduler 策略构造边界，当前只保留 `Simple` 行为。
- `ExecutorEventFactory`：统一构造 `ExecutorEvent` 生命周期事件和 `TaskResult`。
- `AgentStatusTransitionPolicy`：集中表达 agent runtime status/activity transition。
- `Kernel::new`：仍可调用，但已标记 deprecated，作为兼容入口和迁移期 grep 目标。
- `SimpleScheduler`：仍可直接使用，但已标记 deprecated，作为兼容入口和迁移期 grep 目标。

当前源码扫描结果：

- 生产代码仍直接调用 `Kernel::new`：
  - `macaca/crates/macaca-web/src/lib.rs`
  - `macaca/crates/macaca-app/src/runtime.rs`
  - `macaca/crates/macaca-app/src/workflow.rs`
  - `macaca/crates/macaca-sdk/src/registry_api.rs`
  - `macaca/crates/macaca-cli/src/commands.rs`
- 测试代码仍直接调用 `Kernel::new`：
  - `macaca/crates/macaca-kernel/tests/e2e_auto_programming.rs`
  - `macaca/crates/macaca-integration-tests/tests/app_declarative.rs`
  - `macaca/crates/macaca-integration-tests/tests/fullstack_autodev.rs`
  - `macaca/crates/macaca-integration-tests/tests/live_fullstack_autodev.rs`
  - `macaca/crates/macaca-integration-tests/tests/kernel_lifecycle.rs`
- `SimpleScheduler` 直接调用基本只在 `macaca-kernel` 自身兼容测试和 `SchedulerFactory` 内部 bridge 中出现。
- `macaca-web/src/loop_manager.rs` 已经通过 `ExecutorEventFactory` 构造 executor lifecycle event，不需要再次迁移。

本轮不处理 `macaca-task::TaskResult`、`macaca-proto::TaskResult`、`DelegatedTaskResult` 等同名类型；它们不是本次 `macaca-kernel::executor::TaskResult` 的 deprecated 迁移目标。

## 设计模式适配

本轮只迁移上层消费方到已有设计模式边界，不引入新抽象：

- **Builder**：上层统一使用 `KernelBuilder::new(config, llm, tools).build()` 构造 `Kernel`。
- **Factory / Strategy**：scheduler 仍通过 `KernelBuilder` 间接使用 `SchedulerFactory::build(SchedulerKind::Simple)`，上层不直接依赖 `SimpleScheduler`。
- **Facade**：上层继续只依赖 `Kernel` 对外行为，不访问 registry/scheduler/status 内部构造细节。
- **Factory Method**：executor lifecycle event 已经通过 `ExecutorEventFactory` 下沉到 kernel，web 只保留必要的薄 helper。

## 可选方案

### 方案 A：只替换生产代码里的 `Kernel::new`

做法：

- 把 `macaca-web`、`macaca-app`、`macaca-sdk`、`macaca-cli` 的生产路径全部改为 `KernelBuilder`。
- 测试中的 `Kernel::new` 暂时保留。

优点：

- 改动最小。
- 能消除最重要的生产路径 deprecated 调用。
- 风险较低。

缺点：

- 测试仍会产生 deprecation warning。
- 无法形成“上层代码禁止 deprecated kernel 构造”的完整规则。
- 后续可能继续复制测试里的旧写法到生产代码。

结论：不推荐作为最终方案，可作为实施第一步。

### 方案 B：生产路径迁移 + 上层测试迁移 + kernel 内兼容测试保留

做法：

- 生产代码全部改用 `KernelBuilder`。
- 上层测试中的普通 helper 也改用 `KernelBuilder`。
- `macaca-kernel` 自身保留少量 `Kernel::new` / `SimpleScheduler` 兼容测试，并显式 `#[allow(deprecated)]`。
- 增加 grep 验证：上层生产代码不得直接调用 `Kernel::new` 或 `SimpleScheduler`。
- OpenSpec 明确 deprecated API 的允许范围：kernel 内部兼容层和兼容测试允许，上层生产代码不允许。

优点：

- 符合 additive-first 与 1:1 行为还原。
- 生产代码不再依赖 deprecated kernel API。
- 测试代码也大部分体现新入口，降低未来复制旧写法的风险。
- 保留 kernel 兼容测试，确保 deprecated API 在迁移期仍可用。

缺点：

- 触达 crate 较多，需要更完整的 cargo check/test。
- 需要在 grep 规则中区分“禁止的上层调用”和“允许的 kernel 兼容测试”。

结论：推荐。

### 方案 C：彻底禁止所有 `Kernel::new` / `SimpleScheduler` 调用

做法：

- 所有生产代码、测试代码、kernel 内部兼容测试全部改用新入口。
- 只保留 deprecated 定义，不再有任何调用点。

优点：

- 调用面最干净。
- grep 结果最简单。

缺点：

- 旧 API 虽然保留，但没有测试证明它仍可调用。
- 与“deprecated 但不要删除，便于后续迁移查找”的项目策略冲突。
- 如果外部用户仍依赖旧 API，兼容性破坏不容易被发现。

结论：不推荐。

## 推荐方案

采用方案 B：生产路径迁移 + 上层测试迁移 + kernel 内兼容测试保留。

迁移边界：

- `macaca-web`：启动 kernel 时使用 `KernelBuilder`，保持 LLM router、toolset、app registry 逻辑不变。
- `macaca-app`：runtime/workflow 测试 helper 使用 `KernelBuilder`，不改变 workflow 行为。
- `macaca-sdk`：registry API 测试 helper 使用 `KernelBuilder`，不改变 register behavior。
- `macaca-cli`：CLI kernel 创建路径统一走一个本地 `build_kernel` helper，内部使用 `KernelBuilder`，避免四处重复构造。
- `macaca-integration-tests`：普通测试 helper 改用 `KernelBuilder`。
- `macaca-kernel`：保留 `Kernel::new` compatibility test 和 `SimpleScheduler` compatibility tests，用局部 `#[allow(deprecated)]` 表达这是兼容覆盖。

## 风险与控制

- 风险：`macaca-web` 启动路径受影响，前后端无法启动。
  控制：只替换构造函数，不改 LLM、tools、registry、session、trace、task loop；验证 `cargo check -p macaca-web`。

- 风险：`macaca-cli` 多个 command 复制 kernel 构造逻辑，局部替换可能遗漏。
  控制：先抽一个 crate-local helper `build_kernel(config, llm, tools)`，所有 command 复用该 helper。

- 风险：测试中完全移除 deprecated 调用会失去兼容覆盖。
  控制：只在 `macaca-kernel` 自身保留 compatibility coverage，上层测试迁移到新入口。

- 风险：grep 误伤同名类型或 kernel 内部兼容层。
  控制：grep 分两类执行：生产上层禁止；kernel 内部定义/兼容测试允许。

- 风险：GitNexus 当前索引可能未覆盖最新未提交文件。
  控制：实施前对要编辑的函数跑 `gitnexus_impact`；提交前跑 `gitnexus_detect_changes`，提交后重建索引。

## 成功标准

- 上层生产代码中没有直接调用 `Kernel::new`。
- 上层生产代码中没有直接调用 `SimpleScheduler`。
- `macaca-web`、`macaca-app`、`macaca-sdk`、`macaca-cli` 使用 `KernelBuilder` 构造 kernel。
- 普通 integration tests 使用 `KernelBuilder`。
- `Kernel::new` 和 `SimpleScheduler` 仍保留且 deprecated，不删除。
- deprecated 兼容调用只出现在 `macaca-kernel` 内部兼容层、factory bridge、明确的 compatibility tests 中。
- 不改变 scheduler 行为、executor event payload、trace/EventLog/SSE、planner/worker/coordinator、driver、skill、MCP 行为。
- 不引入任何 application/workflow/driver/agent 名称硬编码。
- 不引入新依赖。
