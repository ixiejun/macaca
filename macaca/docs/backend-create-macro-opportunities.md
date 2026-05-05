# Macaca 后端 create 宏化机会评估

日期：2026-05-04

## 目标

全面扫描 `macaca/crates` 后端 Rust 代码里与 `create` / `create_*` / `create*` 相关的创建逻辑，评估哪些重复结构适合用 Rust 声明宏或过程宏收敛，哪些必须继续保留显式业务代码。

本文件只记录探索结论和后续落地建议，不改变现有行为。

## 探索方法

- 扫描范围：`macaca/crates/**/*.rs`。
- 关键检索：`create`、`create_*`、`create*`、`macro_rules!`、`proc_macro`。
- 重点阅读：任务创建、工具创建、HTTP create endpoint、driver/runtime/ipc factory、workspace tools、已有宏。
- 设计约束：参考 `macaca/docs/design_patterns.md`，优先使用 Factory、Builder、Command、Facade、Template Method 等显式设计模式；只有结构性重复稳定时才引入宏。

## 当前已有宏

| 宏 | 位置 | 作用 | 评价 |
| --- | --- | --- | --- |
| `export_driver!` | `macaca-driver/src/sdk.rs` | 为 driver 插件生成 C ABI 导出符号 | 合理。ABI 样板稳定且重复度高，声明宏比手写更安全。 |
| `tool_schema!` | `macaca-framework/src/tool.rs` | 生成工具参数 JSON Schema | 方向正确，但 DSL 仍偏底层，未覆盖 typed input / alias / validation。 |
| `subscriber!` | `macaca-kernel/src/executor/bus.rs` | 简化事件订阅 helper | 合理，属于稳定结构胶水。 |
| `trace_agent_reply!` / `trace_model_chat!` / `trace_tool_exec!` | `macaca-framework/src/tracing_utils.rs` | 简化 trace event 构造 | 合理，可继续扩展为结构化 event helper。 |

未发现已落地的过程宏。

## create 代码分类

### 1. 不建议宏化：领域创建流程

这些函数包含业务规则、状态转移、持久化语义或异步编排。宏化会隐藏控制流，降低可审查性。

| 位置 | 符号 | 当前模式 | 建议 |
| --- | --- | --- | --- |
| `macaca-task/src/todo_board.rs` | `TaskSpace::create_task_assignment` | Factory Method + Domain Service | 保持显式。后续可引入 `TaskAssignmentBuilder` 降低长参数风险。 |
| `macaca-task/src/tracker.rs` | `TaskTracker::create` | Repository / Service | 保持显式。逻辑短，不值得宏化。 |
| `macaca-task/src/scheduler.rs` | `TaskScheduler::create` | Repository / Scheduler Service | 保持显式。调度状态和持久化应可读。 |
| `macaca-framework/src/plan.rs` | `PlanNotebook::create_plan` | Aggregate Root / Builder-like API | 保持显式。计划状态机不应藏进宏。 |
| `macaca-web/src/loop_manager.rs` | `create_goal` | HTTP Handler + Orchestration Facade | 不宏化。可抽取 request DTO 和 service facade。 |
| `macaca-web/src/loop_manager.rs` | `create_fallback_decomposition_tasks` | Strategy / Template Method 候选 | 不宏化。可抽策略对象，避免把 fallback 规则写成宏。 |
| `macaca-kernel/src/executor/fork_manager.rs` | `create_fork` | Orchestration Service | 不宏化。fork 生命周期和错误路径必须显式。 |

理由：

- 这些 create 不是样板，而是系统行为。
- Macaca 是基础设施，create 路径通常决定 session、app、agent、trace、task 生命周期，宏会让调用方难以判断副作用。
- 更合适的优化是 Builder、Facade、Policy、Strategy、Repository，不是宏。

### 2. 适合声明宏：稳定 JSON/schema/event 样板

这些重复主要是结构性样板，声明宏足够，且不会引入过程宏编译开销。

| 位置 | 重复类型 | 机会 | 推荐级别 |
| --- | --- | --- | --- |
| `macaca-tools/src/todo.rs` | `parameters_schema` 手写 JSON、required 字段、默认值 | 扩展 `tool_schema!` DSL，支持 required/optional/default/array/object/enum | 高 |
| `macaca-web/src/framework_toolkit.rs` | workspace tools 的 schema JSON 重复 | 复用统一 schema DSL | 高 |
| `macaca-tools/src/builtin.rs` | built-in tools schema 和输入字段提取 | schema DSL + input alias helper | 中 |
| `macaca-web/src/run_trace.rs` 及调用点 | event payload JSON 构造重复 | `trace_payload!` / `emit_trace!` 声明宏或 helper function | 中 |
| `macaca-web/src/bootstrap.rs` | route 注册重复 | 小型 route group helper 或函数，优先函数，必要时声明宏 | 低 |
| 各 crate 单测 | `create_test_*` fixture 重复 | `test_fixture!` 声明宏或 builder helper | 低 |

建议先升级声明宏而不是上过程宏：

```rust
tool_schema! {
    required {
        agent: string("Target assignee agent name"),
        title: string("Short task title"),
        description: string("Detailed task description"),
    }
    optional {
        priority: integer("Priority 0-10").default(5),
        depends_on: array(string("Task id")),
    }
}
```

该方向的收益：

- 保留 schema 在调用点附近，审查成本低。
- 不需要新 crate，不引入过程宏编译链路。
- 能先覆盖最明显重复，风险低。

### 3. 适合过程宏：Tool trait 结构生成

过程宏适合在工具系统完成 typed input 之后再引入。当前大量工具直接处理 `serde_json::Value`，如果直接上过程宏，容易把 validation、alias、错误消息和业务执行耦合到宏里。

推荐目标形态：

```rust
#[derive(MacacaToolInput)]
struct CreateTodoInput {
    #[tool(required, description = "Target assignee agent name")]
    agent: String,
    #[tool(required, description = "Short task title")]
    title: String,
    #[tool(default = 5, description = "Priority 0-10")]
    priority: u8,
}

#[macaca_tool(
    name = "create_todo",
    description = "Create a new task and assign it to a specific agent's task board."
)]
impl CreateTodoTool {
    async fn run(&self, input: CreateTodoInput) -> MacacaResult<Value> {
        // 业务逻辑仍然显式写在这里
    }
}
```

适用范围：

- `impl Tool for XxxTool` 的 `name` / `description` / `parameters_schema` / `execute` 样板。
- typed input 到 JSON Schema 的 derive。
- 字段 alias、required、default、enum、array/object schema。

不适用范围：

- 不生成业务规则。
- 不生成持久化逻辑。
- 不生成 orchestration 分支。
- 不把 app/workflow/agent/driver 名称写进宏属性。

建议只有满足以下条件才创建 `macaca-tools-macros` 或 `macaca-framework-macros`：

- 至少 5 个以上工具迁移到 typed input。
- schema 合约已经通过 OpenSpec 固化。
- 错误消息、alias、默认值策略已经统一。
- 编译时间和 IDE 诊断成本可接受。

## 重点文件评估

### `macaca-tools/src/todo.rs`

`CreateTodoTool::create_one` 包含大量输入解析、依赖解析、去重、foundation dependency 推断和 task 创建。这里最容易误判为“宏化候选”，但核心逻辑不应宏化。

可改进点：

- 用 typed input struct 替代散落的 `input["field"]`。
- 用声明宏或 derive 生成 `parameters_schema`。
- 用 Builder 收敛 `create_task_assignment` 的长参数。
- `create_one` 可拆成 parse、validate、resolve dependencies、persist、response mapping 几个私有函数。

推荐模式：

- Command：`CreateTodoTool` 是工具命令。
- Builder：构造 `TaskAssignmentRequest`。
- Policy：disallowed assignee、dedup、dependency inference。
- Macro：只负责 schema / input metadata。

### `macaca-web/src/framework_toolkit.rs`

workspace `file_read` / `file_write` / `shell` 的 `impl Tool` 样板重复明显：

- `name`
- `description`
- `parameters_schema`
- `normalize_tool_input`
- alias 字段提取
- JSON response shape

可优先落地声明宏或 typed input derive。这里比 `CreateTodoTool` 更适合作为第一批试点，因为业务逻辑较薄，风险低。

推荐顺序：

1. 先统一 input alias helper。
2. 再用 schema DSL 替换手写 JSON schema。
3. typed input 稳定后，再评估过程宏生成 `Tool` impl。

### `macaca-driver/src/sdk.rs`

`export_driver!` 已经是合理的声明宏使用案例。driver ABI 导出函数多、重复稳定、错误边界一致，宏化收益高。

后续可选优化：

- 拆出内部 helper 函数，减少宏展开体积。
- 为 create closure 提供更清晰的 typed config adapter。
- 不建议改成过程宏，除非需要从 `SoftwareDriver` impl 自动读取 manifest 元数据。

### transport / factory create

涉及文件：

- `macaca-driver/src/factory.rs`
- `macaca-ipc/src/transport.rs`
- `macaca-ipc/src/local.rs`
- `macaca-ipc/src/nats.rs`
- `macaca-runtime-host/src/transport.rs`

这些是标准 Factory / Abstract Factory 结构，代码短且语义清晰。宏化收益低。

建议：

- 保持 trait factory。
- 如果实现数量继续增长，可用注册表或 builder 收敛配置选择。
- 不建议用宏生成 `create_sender` / `create_receiver` / `create_client`。

### HTTP create endpoint

涉及：

- `macaca-web/src/routes.rs::create_schedule`
- `macaca-web/src/loop_manager.rs::create_goal`

问题主要不是宏，而是 handler 同时负责解析、校验、调用 domain service、启动 loop、响应映射。

推荐：

- 用 request DTO + service facade。
- 用 `FromRequest` / typed extractor 或普通 helper 统一错误响应。
- 只有 route 注册有大量重复时，才考虑小型声明宏。

## 宏化优先级

| 优先级 | 方向 | 宏类型 | 原因 | 风险 |
| --- | --- | --- | --- | --- |
| P0 | 扩展 `tool_schema!` 或新增 `tool_schema_v2!` | 声明宏 | 低风险，高重复收益 | DSL 设计过度会难读 |
| P1 | 工具 typed input schema derive | 过程宏 | 长期收益高，可减少 JSON 手写和解析错误 | 编译时间、诊断、宏属性设计成本 |
| P1 | workspace/builtin tools 的 Tool impl 样板 | 过程宏或声明宏 | 工具数量增长后收益明显 | 过早抽象会影响调试 |
| P2 | trace event payload helper | 声明宏或函数 | 降低事件字段遗漏 | 宏不如函数易调试 |
| P3 | test fixture builder | 声明宏或 helper function | 单测可读性提升 | 宏收益有限 |
| 不做 | task/goal/fork/schedule 业务 create 流程宏化 | 无 | 业务语义必须显式 | 宏会隐藏副作用 |

## 推荐实施路线

### 阶段一：无过程宏，先收敛结构重复

- 扩展或新增 `tool_schema!` DSL。
- 迁移低风险工具 schema：workspace tools、builtin file tools。
- 引入 typed helper 函数处理 input alias 和 required field。
- 保持 `execute` 业务逻辑显式。

适用设计模式：

- Facade：统一 schema 构造入口。
- Template Method：工具执行流程可保持 `execute -> parse -> run -> response`。
- Command：每个 tool 仍然是独立命令对象。

### 阶段二：引入 typed input，不急于过程宏

- 为高频工具增加 input DTO，例如 `CreateTodoInput`、`FileReadInput`、`ShellInput`。
- DTO 负责 `TryFrom<Value>`，错误消息统一。
- schema 仍可由声明宏提供，避免一次性引入过程宏。

适用设计模式：

- Builder：复杂 create request。
- Adapter：`serde_json::Value` 到 typed input。
- Policy：校验和默认值策略。

### 阶段三：过程宏试点

仅当 typed input 和 schema 合约稳定后，引入过程宏 crate。

建议命名：

- `macaca-tools-macros`：如果只服务 `macaca-tools::Tool`。
- `macaca-framework-macros`：如果要服务更通用 framework tool abstraction。

首批试点：

- `WorkspaceFileReadTool`
- `WorkspaceFileWriteTool`
- `WorkspaceShellTool`

暂不试点：

- `CreateTodoTool`
- `CreateGoalTool`
- fork / planner / scheduler 相关 create。

### 阶段四：再评估复杂工具

当过程宏在简单工具上稳定后，再迁移 `CreateTodoTool` 的外围样板：

- `name`
- `description`
- `parameters_schema`
- typed input parse

业务步骤仍然显式拆分，不让宏生成。

## 风险清单

- 宏隐藏控制流：create 路径涉及生命周期和持久化，宏会增加审查成本。
- 过程宏诊断较差：错误位置可能指向展开代码，影响日常开发效率。
- 编译时间增加：过程宏 crate 会增加增量编译成本。
- schema 合约漂移：宏生成 schema 后，必须有测试锁定 tool schema。
- 过度 DSL：声明宏语法如果过复杂，会比 JSON 更难读。
- 基础设施通用性受损：宏属性里禁止硬编码 app/workflow/agent/driver/business 名称。
- 迁移风险：旧工具 schema 是 LLM/上层调用契约，字段 required/default/alias 变化必须走 OpenSpec。

## 结论

Macaca 后端当前最值得宏化的不是领域 `create` 本身，而是 create 周边的稳定样板：

- 工具 schema 构造。
- typed input 元数据。
- Tool trait 实现胶水。
- trace event payload。
- 少量测试 fixture。

任务、goal、fork、schedule、transport、driver factory 等 create 流程应继续采用显式设计模式。宏只应作为结构化样板的生成工具，不应承载业务规则。

推荐下一步先做 OpenSpec 提案，范围限定为“工具 schema DSL 与 typed input adapter”，避免一次性引入过程宏；等简单工具迁移完成并有 schema 契约测试后，再评估过程宏。
