# Route C 服务化依赖门禁 Allowlist

## 1. 目的

本文档记录 S0 依赖边界门禁中暂时允许通过的既有依赖债务。

Allowlist 不是架构批准。它只是当前迁移状态的 Memento：说明哪条依赖违反 Route C 微内核边界、为什么还存在、未来应该迁到哪个 service/facade、在哪个阶段过期。

新增例外必须先更新 OpenSpec，并同步更新 `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries.rs` 中的测试内 allowlist。禁止只改代码或只改文档。

## 2. 执行门禁

可执行门禁位于：

- `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries.rs`

运行方式：

```bash
cargo test -p macaca-integration-tests route_c_dependency_boundaries
```

门禁使用 `cargo metadata --no-deps --format-version 1` 读取 workspace 直接依赖边，并按 Route C layer 评估 forbidden rules。未知 workspace crate 会失败，要求先通过 OpenSpec 明确分层。

## 3. Allowlist 字段

| 字段 | 含义 |
| --- | --- |
| Rule id | 触发的边界规则 |
| From crate | 依赖发起 crate |
| To crate | 被依赖 crate |
| Current reason | 当前保留原因 |
| Replacement service/facade path | 目标替代路径 |
| Target migration phase | 计划迁移阶段 |
| Expiry condition | 何时删除该 allowlist |
| Owner/status | 责任归属和当前状态 |

## 4. 当前迁移债务

| Rule id | From crate | To crate | Current reason | Replacement service/facade path | Target migration phase | Expiry condition | Owner/status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `kernel-no-provider-deps` | `macaca-kernel` | `macaca-driver` | Kernel 当前仍承载 driver execution 兼容路径。 | Driver Service facade | S6 | Driver 调用全部经 ServiceRuntime 或 SystemFacade。 | Route C / active debt |
| `kernel-no-provider-deps` | `macaca-kernel` | `macaca-gateway` | Kernel 当前仍保留 gateway 兼容注册和协调入口。 | Gateway Service facade | S8 | Gateway provider 通过 service/plugin 注册，不再由 kernel 直接依赖。 | Route C / active debt |
| `kernel-no-provider-deps` | `macaca-kernel` | `macaca-llm` | Kernel 当前仍保留 LLM provider 兼容路径。 | LLM Service facade | S5 | LLM 调用全部经 service contract。 | Route C / active debt |
| `kernel-no-provider-deps` | `macaca-kernel` | `macaca-memory` | Kernel 当前仍保留 memory/context 兼容路径。 | Memory/Context Service facade | S5 | 记忆和上下文召回全部经 Memory/Context Service。 | Route C / active debt |
| `kernel-no-provider-deps` | `macaca-kernel` | `macaca-persist` | Kernel 当前仍直接使用持久化能力保存系统状态。 | Persistence Service contract | S1/S2 | Kernel 只依赖 persistence contract 或 service facade。 | Route C / active debt |
| `kernel-no-provider-deps` | `macaca-kernel` | `macaca-skill` | Kernel 当前仍保留 skill/MCP 兼容路径。 | Skill/MCP Service facade | S6 | Skill/MCP 通过 service/plugin runtime 接入。 | Route C / active debt |
| `kernel-no-provider-deps` | `macaca-kernel` | `macaca-task` | Kernel 当前仍保留 task primitive 与 task service 混合路径。 | Task Service facade | S4 | Planner/worker/review 全部迁到 Task Service。 | Route C / active debt |
| `kernel-no-provider-deps` | `macaca-kernel` | `macaca-tools` | Kernel 当前仍保留 tools 调用兼容入口。 | Tool/Skill Service facade | S6 | 工具能力通过 Skill/Tool Service 调用。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-cli` | `macaca-gateway` | CLI 当前仍包含 gateway 启动/检查兼容入口。 | Gateway Service client | S8 | CLI 只通过 SDK/SystemFacade 调 gateway service。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-cli` | `macaca-llm` | CLI 当前仍存在 LLM pipeline 兼容调用。 | LLM Service client | S5 | CLI 不再直接依赖 LLM provider crate。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-cli` | `macaca-tools` | CLI 当前仍直接访问 tools 兼容能力。 | Tool/Skill Service client | S6 | CLI 工具调用全部经 service client。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-web` | `macaca-driver` | Web 当前仍是部分 driver execution 的协调入口。 | Driver Service client | S6 | Web 只订阅 driver trace/state，不构造 driver provider。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-web` | `macaca-llm` | Web 当前仍保留 chat/pipeline LLM 兼容路径。 | LLM Service client | S5 | `/api/chat/v2` 经 SystemFacade/service client 调用。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-web` | `macaca-memory` | Web 当前仍直接读取部分 memory/session trace 数据。 | Memory/Context Service client | S5 | Web 只请求 session scoped memory/context view。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-web` | `macaca-persist` | Web 当前仍直接访问持久化层读取 session/application 状态。 | Persistence Service client | S1/S12 | Web 通过 facade 拉取分页 session 和 trace。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-web` | `macaca-skill` | Web 当前仍直接处理 skill/MCP 展示和调用兼容路径。 | Skill/MCP Service client | S6 | Web 只展示 Skill/MCP service state 和 trace。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-web` | `macaca-task` | Web 当前仍直接访问 task board/session task 状态。 | Task Service client | S4 | Task board 通过 Task Service 分页 API 获取。 | Route C / active debt |
| `presentation-no-provider-construction-hub` | `macaca-web` | `macaca-tools` | Web 当前仍直接依赖 tools 兼容能力。 | Tool/Skill Service client | S6 | Web 工具相关展示通过 Skill/Tool Service view model。 | Route C / active debt |
| `cli-no-web-internals` | `macaca-cli` | `macaca-web` | CLI 当前复用 Web server startup 兼容路径。 | shared shell facade in `macaca-sdk` | S12 | Web/CLI thin shell 完成后，CLI 不再依赖 Web crate。 | Route C / active debt |

## 5. 新增例外流程

1. 创建或更新 OpenSpec change，解释为什么短期不能立即迁移。
2. 在本文件新增 allowlist 行，必须填写迁移阶段和过期条件。
3. 在依赖门禁测试的 test-local allowlist 中新增同等记录。
4. 运行 `openspec validate <change-id> --strict`。
5. 运行 `cargo test -p macaca-integration-tests route_c_dependency_boundaries`。

优先删除 allowlist 行，而不是延长例外。任何无法说明替代 service/facade path 的新增依赖，都不应进入 Route C 主线。
