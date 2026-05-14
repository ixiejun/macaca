<!-- OPENSPEC:START -->
# OpenSpec Instructions

These instructions are for AI assistants working in this project.

Always open `@/openspec/AGENTS.md` when the request:
- Mentions planning or proposals (words like proposal, spec, change, plan)
- Introduces new capabilities, breaking changes, architecture shifts, or big performance/security work
- Sounds ambiguous and you need the authoritative spec before coding

Use `@/openspec/AGENTS.md` to learn:
- How to create and apply change proposals
- Spec format and conventions
- Project structure and guidelines

Keep this managed block so 'openspec update' can refresh the instructions.

<!-- OPENSPEC:END -->

## 最重要的事
在和我进行交互，思考过程，输出总结，非编码输出请用简体中文！

## Macaca Architecture — MANDATORY

Macaca is a **generic Agent OS platform**, NOT a specific application. The `macaca-*` crates are the OS foundation that must support ANY application.

### Rules

1. **NEVER hardcode application-specific logic in OS crates** — no agent names ("backend", "frontend", "coordinator"), no routing strategies, no prompts, no model names in `macaca-kernel`, `macaca-runtime`, `macaca-task`, `macaca-llm`, `macaca-web`, `macaca-tools`, `macaca-proto`, `macaca-persist`
2. **Application behavior is configured via persona files** (`personas/{agent}/IDENTITY.md`, `TOOLS.md`, `SOUL.md`, etc.) and config files (`config/default.toml`), never hardcoded in source
3. **Self-check before writing OS code**: "If someone creates a quantitative trading application instead of fullstack-autodev, would this code still work?" If no → refactor to be application-agnostic
4. **`fullstack-autodev` is a test application** in `examples/apps/` — it tests the OS but does not define it

### Layer Separation

| Layer | Location | Contains |
|-------|----------|----------|
| OS Foundation | `macaca/crates/macaca-*` | Generic scheduling, execution, persistence, LLM abstraction |
| Application Config | `examples/apps/{name}/personas/` | Agent identity, tools, routing strategy |
| User Config | `config/default.toml` | Ports, models, API keys, budgets |

### Project Structure

```
agent/                          # 项目根目录
├── frontend/                   # 前端 — Next.js 16 + React 19 + Tailwind CSS
│   ├── app/                    #   Next.js App Router 页面
│   │   └── chat/               #   聊天页面
│   ├── components/             #   UI 组件 (TaskBoardModal, AgentPanel, Sidebar 等)
│   ├── lib/                    #   API 客户端 (api.ts) + 类型定义 (types.ts)
│   └── package.json
├── macaca/                     # 后端 — Rust workspace
│   ├── crates/                 #   OS 底座 crate 集合
│   │   ├── macaca-web/         #     Axum HTTP API + SSE (端口 3001)
│   │   ├── macaca-kernel/      #     执行器、队列、Fork 管理
│   │   ├── macaca-task/        #     TaskBoard, TodoStore, PlanLoop, WorkerLoop
│   │   ├── macaca-runtime/     #     Agentic Loop, 权限, 上下文窗口
│   │   ├── macaca-proto/       #     共享类型 (TodoItem, Task, Config)
│   │   ├── macaca-llm/         #     LLM 抽象 + Router + 降级
│   │   ├── macaca-tools/       #     Agent 工具集 (orchestration, builtin)
│   │   ├── macaca-persist/     #     redb 持久化
│   │   └── ...                 #     gateway, cli, app, agent, memory 等
│   ├── config/                 #   运行时配置 (default.toml)
│   └── examples/apps/          #   示例应用 (fullstack-autodev)
└── openspec/                   # OpenSpec 规范管理
```

**前后端通信**: 前端通过 `lib/api.ts` 调用后端 REST API (端口 3001)，实时事件通过 SSE 推送。

---

## Development Principles & Best Practices

> 从设计文档、实现计划、已修复 bug、代码审计报告中提炼的完整开发指南。

### Project Identity

Macaca 是一个 Agent 操作系统底座，目标是 7x24 小时自主运行、零人工干预的智能代理平台。

| 维度 | 说明 |
|------|------|
| 核心隐喻 | Agent = 进程，Kernel = 运行时+调度器，Syscall = LLM/工具调用 |
| 灵感来源 | Linux 进程模型, K8s 声明式状态, OpenFang Hands, memU 三层记忆, OpenClaw 异步解耦 |

### Architecture Principles

1. **Persona-Driven Behavior** — Agent 行为通过 persona 文件 (`IDENTITY.md`, `TOOLS.md`, `SOUL.md`) 配置，不在源码定义
2. **Event-Driven PlanLoop** — PlanLoop 不直接调用 LLM，通过 `PlanEvent` 通道将决策委托给消费者（解耦调度与执行）
3. **Pull-Based WorkerLoop** — Worker Agent 自主从 TaskBoard 拉取任务（灵感来自 K8s 控制器模式）
4. **Independent Key Storage** — 每个 Todo/Task 使用独立 key 持久化，避免大 JSON 读写竞争。每次状态变更立即持久化
5. **Callback Pattern** — 使用 `Box<dyn Fn(...) + Send + Sync>` 回调避免跨 crate 循环依赖
6. **Three-Layer Isolation** — System → Application → Agent，各层完全隔离
7. **Dual Execution Model** — 即时委托 (`delegate_task`) vs 项目级 (`create_goal`)，由 LLM 自然选择

### Development Best Practices

#### Explore Before Coding
- 使用 GitNexus `gitnexus_impact` 检查影响范围后再修改
- 使用 `gitnexus_context` 了解上下游依赖
- 阅读 persona 文件确认行为预期

#### Debug with Evidence
- 后端日志: `tail -f /tmp/macaca-backend.log`
- 前端调试: Chrome DevTools (`mcp__chrome-devtools__*`) + Playwright
- 调试链路: 用户输入 → Coordinator → PlanLoop/delegate_task → WorkerLoop → Execution → Result

#### Verify After Every Change
```bash
cargo check                      # 编辑后立即检查
cargo test -p macaca-<crate>     # 实现后测试
cargo check                      # 全 workspace 验证
```

#### Test at the Right Layer
| 层级 | 测试类型 | 示例 |
|------|----------|------|
| 纯逻辑 | 单元测试 | `LoopDetector`, `CostTracker` |
| 跨模块 | 集成测试 | TaskBoard → TodoStore → RedbStore |
| 全链路 | E2E 测试 | create_goal → 分解 → 执行 → 审查 → 完成 |

### Common Pitfalls — 已踩过的坑

| 问题 | 根因 | 教训 |
|------|------|------|
| 模块已实现但功能不生效 | 未接入运行时 (AppState/agent_runner) | **模块已编写 ≠ 已接入运行时 ≠ LLM 能使用** |
| LLM 忽略 routing guide | system prompt 在中间函数被覆盖 | 检查 prompt 的完整传递链路 |
| 工具注入了但 LLM 看不到 | tool_defs 在错误的层构建 | 验证 `to_definitions()` 返回值 |
| Worker 完成任务但状态不更新 | delegate_task 后无回调更新 TaskBoard | 每个异步操作完成后必须有状态回写 |
| PlanLoop 未启动 | 仅在 HTTP 端点启动，工具调用路径遗漏 | 用回调 `on_created` 保证任何路径都触发 |
| 硬编码 agent 名/prompt | 底座代码包含应用专有逻辑 | 自检："换个 application 还能工作吗？" |

### Debugging Checklist

当功能不工作时，按此顺序排查:
1. 模块是否已实现？(源文件存在)
2. 模块是否已导出？(`lib.rs` 中 `pub mod` + `pub use`)
3. 模块是否已接入运行时？(`AppState`/`agent_runner.rs`)
4. 工具是否已注入？(`build_agent_toolset()` 中的 `extra`)
5. LLM 是否能看到工具？(`to_definitions()`)
6. Persona 是否引导正确？(`TOOLS.md`)
7. 事件是否被消费？(PlanEvent/WorkerEvent 消费者)

### Key Design Patterns

| 模式 | 文件 | 说明 |
|------|------|------|
| ResilientLlmWrapper | `macaca-llm/src/resilient.rs` | 重试+退避+预算+降级链 |
| LoopDetector | `macaca-runtime/src/loop_detector.rs` | SHA256 哈希检测重复调用，三级响应 |
| ContextWindowManager | `macaca-runtime/src/context_window.rs` | Token 估算+截断，保留 system msg + 最近 N 轮 |
| WorkerSupervisor | `macaca-kernel/src/executor/app_executor.rs` | 自动重启 (max 5 次) + cooldown 重置 |
| Crash Recovery | `macaca-task/src/todo_store.rs` | IN_PROGRESS/ASSIGNED 回滚为 PENDING |
| AgentToolSet | `macaca-web/src/agent_runner.rs` | base tools (全局) + extra tools (per-agent) |

### Technology Decisions

| 维度 | 选型 | 理由 |
|------|------|------|
| 核心语言 | Rust | 内存安全、并发安全 (Send+Sync) |
| 持久化 | redb | 纯 Rust、ACID、嵌入式 |
| 默认 LLM | DashScope/qwen3-max | 中文语义好、性价比高 |
| Web 框架 | Axum | 类型安全、Tower 中间件 |
| 部署 | systemd | WatchdogSec + 自动重启 |
| 监控 | Prometheus + AlertManager | 标准指标 + webhook 告警 |

### Port Configuration

| 服务 | 端口 |
|------|------|
| Frontend (Next.js) | 3000 |
| Backend (Macaca Rust API) | 3001 |

### Continuous Learning

**此文档应在后续开发中不断沉淀新的经验**。每次遇到新的 bug、设计决策、或最佳实践，都应追加到对应章节，而非仅靠对话记忆。
<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **agent** (30994 symbols, 57868 relationships, 300 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/agent/context` | Codebase overview, check index freshness |
| `gitnexus://repo/agent/clusters` | All functional areas |
| `gitnexus://repo/agent/processes` | All execution flows |
| `gitnexus://repo/agent/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
