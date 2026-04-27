# Agent OS 级 MCP Runtime 研究与实现计划

## 背景

目标是在 `macaca-framework` 中完整支持 MCP 协议，并把 MCP 能力整合到 Macaca Agent OS：MCP 服务由 Agent OS 安装、注册和管理后，所有 application 都可以按策略调用 MCP 工具。

这份文档是研究与计划，不是 OpenSpec 提案。确认方向后，再为第一阶段实现编写 OpenSpec。

## AgentScope MCP 设计要点

AgentScope 的 MCP 支撑集中在 `agentscope/src/agentscope/mcp` 和 `Toolkit.register_mcp_client`，核心思路如下：

- `MCPClientBase` 定义统一 client 抽象：按工具名获取 callable function。
- `StatefulClientBase` 管理长连接 MCP session，适合浏览器、IDE、状态型服务；显式 `connect()` / `close()`，工具调用复用同一 session。
- `HttpStatelessClient` 每次工具调用临时建立连接，适合无状态 HTTP MCP 服务。
- `StdIOStatefulClient` 用 stdio 启动本地 MCP 子进程。
- `HttpStatefulClient` 支持 `sse` 和 `streamable_http` 两类 HTTP transport。
- `MCPToolFunction` 把 MCP `Tool` 封装成普通 async callable，并把 MCP content 转换成 framework 内部 `ToolResponse`。
- `Toolkit.register_mcp_client` 负责 `list_tools()`、过滤 enable/disable、处理 preset args、名字冲突策略，并把 MCP 工具注册进普通 toolkit。
- MCP 错误在工具调用层被转成可见的 tool response，而不是让 agent 无上下文失败。

可借鉴的关键不是 Python 实现本身，而是这几个职责边界：`Client` 负责协议连接，`ToolFunction/Handler` 负责工具适配，`Toolkit` 负责注册和冲突策略，外层 Runtime 负责生命周期和策略。

## Macaca 当前状态

当前 Macaca 有三块相关实现：

- `macaca-framework/src/mcp.rs`
  - 已有真实 stdio JSON-RPC MCP client。
  - 已有 `McpClient` trait、`StdioMcpClient`、`McpToolHandler`、`register_mcp_tools`。
  - 目前只覆盖 stdio transport，content 转换偏基础，缺少 SSE / streamable HTTP、资源/提示词等扩展能力。

- `macaca-web/src/skill_mcp.rs`
  - 已能从可见 AgentSkill snapshot 解析 MCP server。
  - 已支持 `playwright-mcp` 兼容注册，并把工具注入 traced framework toolkit。
  - 已有 `skill_mcp_*` EventLog/SSE 事件。
  - 但它是 skill-scoped runtime，不是 OS 级 MCP registry。

- `macaca-mcp` crate
  - 有 `McpTransport`、`McpDriver`、`McpToolAdapter` 形态。
  - 当前 client 仍是 stub 风格，不应作为最终协议实现继续扩展。
  - 后续应选择迁移/废弃/薄封装，而不是和 `macaca-framework/src/mcp.rs` 双轨演进。

## 目标架构

建议把 MCP 分为四层：

1. `macaca-framework` MCP 协议层
   - 提供完整 MCP client abstraction。
   - 支持 stdio、SSE、streamable HTTP。
   - 支持 stateful/stateless 生命周期模型。
   - 统一 MCP content 到 framework `ToolResponse`。
   - 提供 `register_mcp_tools` / namespace / conflict policy。

2. Agent OS MCP Registry
   - 管理系统级 MCP server 安装记录和配置。
   - 配置范围是 OS/global，可被所有 application 复用。
   - application/agent 只做可见性和权限过滤，不负责安装服务。

3. Agent OS MCP Runtime
   - 按 server 配置启动/连接 MCP。
   - 管理进程、连接池、健康检查、超时、资源释放、并发隔离。
   - 支持 stateful 服务按 session/agent 分配实例，避免 Playwright 这类浏览器 profile 冲突。
   - 支持 stateless 服务复用全局连接或按调用建连。

4. Application / Agent Toolkit 注入层
   - `build_toolkit` 从 OS MCP Registry + app/agent policy 解析可见 MCP 工具。
   - MCP 工具以普通 framework tool 进入 agent。
   - 所有调用继续走现有 trace middleware，保证实时 SSE、EventLog、刷新恢复一致。

## 配置模型建议

OS 级 MCP 配置示例：

```yaml
mcpServers:
  playwright:
    transport: stdio
    command: playwright-mcp
    args: ["--headless", "--isolated"]
    lifecycle: session
    stateful: true
    toolPrefix: browser_
    concurrency:
      mode: isolated_per_session
      maxInstances: 4

  context7:
    transport: streamable_http
    url: "http://127.0.0.1:8080/mcp"
    lifecycle: global
    stateful: false
    toolPrefix: context7_
```

Application/agent 只声明策略：

```yaml
tools:
  mcp:
    allowServers: ["playwright"]
    denyTools: ["browser_install"]
```

Skill 可以继续声明 MCP 需求，但不应成为唯一入口：

- Skill metadata 可作为“发现/推荐 MCP server”的来源。
- 真正的安装、可用性、启动参数、并发策略由 Agent OS MCP Registry 决定。

## 关键设计决策

### 1. MCP 能力必须下沉到 framework

所有 agent 都经由 `macaca-framework` 构建和执行，因此 MCP client、tool wrapper、content 转换、tool registration 应该是 framework primitive。`macaca-web` 只负责 OS runtime 和策略 glue。

### 2. OS 级 registry 优先，skill-backed MCP 降级为 discovery

当前 `skill_mcp.rs` 已能证明闭环，但它把 MCP 可用性绑在 skill 可见性上。目标状态应是：

- OS 安装 MCP server。
- 所有 application 默认可发现。
- app/agent policy 决定是否可见。
- skill 只提供使用说明和可选 metadata hint。

### 3. Stateful MCP 必须有隔离策略

浏览器、IDE、终端类 MCP 服务不能简单全局共享同一进程。需要支持：

- `global`：全局长连接，适合无用户状态服务。
- `app`：每个 app 一个实例。
- `session`：每个 session 一个实例。
- `agent_session`：每个 session + agent 一个实例。
- `call`：每次调用临时连接。

Playwright 默认应使用 `session` 或 `agent_session`，并强制 `--isolated` 或唯一 `--user-data-dir`。

### 4. MCP lifecycle 必须可观测

需要统一事件：

- `mcp_server_resolved`
- `mcp_server_starting`
- `mcp_server_ready`
- `mcp_server_failed`
- `mcp_tools_registered`
- `mcp_tool_call`
- `mcp_tool_result`
- `mcp_server_closed`

工具调用本身仍复用现有 `tool_call` / `tool_result` trace，MCP lifecycle 事件用于解释工具为什么出现、失败或释放。

## 分阶段计划

### 阶段 1：framework MCP 协议补齐

目标：把 `macaca-framework/src/mcp.rs` 做成可长期演进的协议层。

任务：

- 抽象 `McpTransportConfig`：`stdio`、`sse`、`streamable_http`。
- 抽象 `McpLifecycleMode`：`stateful` / `stateless`。
- 保留并加固 `StdioMcpClient`。
- 新增 `HttpMcpClient`，支持 SSE 和 streamable HTTP。
- 统一 `McpContent` 转换：text、image、audio、embedded resource，未知类型 fallback 为 JSON 文本。
- 增加 call timeout / list timeout / connect timeout。
- 增加 namespace 和冲突策略：`raise`、`skip`、`prefix`。
- 增加 framework 单元测试和本地 FastMCP 集成测试。

验收：

- stdio MCP 可 list/call/close。
- streamable HTTP MCP 可 list/call。
- SSE MCP 可 list/call。
- MCP 工具注册进 `Toolkit` 后与普通工具一致走 middleware。

### 阶段 2：Agent OS MCP Registry

目标：让 MCP server 成为 OS 基础设施，而不是某个 skill 的附属能力。

任务：

- 新增 OS 级 MCP 配置读取。
- 定义 `McpServerDefinition`、`McpServerPolicy`、`McpServerStatus`。
- 支持全局配置目录和 app overlay。
- 实现配置校验、依赖检查、secret/env redaction。
- 提供 API：列出 MCP servers、工具、状态、失败原因。
- 把现有 `skill_mcp` 兼容 registry 迁移为 MCP discovery source。

验收：

- 安装一个 MCP server 后，所有 app status API 都能看到其可用性。
- app/agent policy 可控制 server/tool 是否进入 toolkit。
- 不需要在 app 里硬编码 Playwright。

### 阶段 3：Agent OS MCP Runtime

目标：可靠启动、复用、隔离、关闭 MCP 实例。

任务：

- 实现 `McpRuntimeManager`。
- 按 lifecycle mode 管理实例 key：global/app/session/agent_session/call。
- 管理引用计数、空闲 TTL、显式 close。
- 对 stateful MCP 支持并发隔离和最大实例数。
- 对 Playwright 默认注入 `--isolated`，必要时生成唯一 `--user-data-dir`。
- 后端 shutdown、session cleanup、agent toolkit drop 时释放 MCP 资源。
- 增加泄漏检测和残留进程清理策略。

验收：

- 两个 session 同时使用 Playwright 不互相抢浏览器 profile。
- session 结束后 Playwright 进程关闭。
- backend 重启后没有 stale MCP instance 被误认为仍可用。

### 阶段 4：Toolkit 注入与 trace 整合

目标：所有 traced framework agent 都可以按策略获得 MCP 工具，且全程可追踪。

任务：

- 在 `framework_toolkit::build_toolkit` 中接入 OS MCP Registry + Runtime。
- MCP 工具按普通 `ToolHandler` 注册，继续经过 trace middleware。
- lifecycle 事件写入 EventLog 和 live SSE。
- 前端可显示 MCP server lifecycle、tool call/result、失败原因。
- 刷新浏览器后可恢复历史 MCP lifecycle 与 tool trace。

验收：

- 新建 session 不刷新即可看到 MCP lifecycle 和 tool trace。
- 刷新后历史 trace 不重复、不丢失。
- MCP 工具失败时，agent tab 显示明确错误，不只返回模型总结。

### 阶段 5：迁移和废弃旧路径

目标：避免双轨 MCP 实现继续发散。

任务：

- `skill_mcp.rs` 改为调用 OS MCP Registry/Runtime，不再直接 spawn MCP。
- `macaca-mcp` crate 做取舍：
  - 要么改成 re-export/wrapper 到 framework MCP。
  - 要么标记 deprecated 并停止新增调用。
- 保留 `metadata.macaca.mcpServers` 解析，但只作为 discovery/config import。
- 更新 `agent-skills-runtime.md`，明确 skill 与 MCP 的职责边界。

验收：

- 代码中只有一个真实 MCP protocol implementation。
- Playwright skill-backed 场景行为不变。
- OS 级 MCP server 不依赖 skill 也能被 app 使用。

## 首个 OpenSpec 建议范围

建议第一份 OpenSpec 只做“framework MCP 协议补齐 + runtime 边界设计”，不要一次性改完 Agent OS registry。

推荐 change id：

```text
add-framework-mcp-transport-runtime
```

## 当前落地状态

`add-agent-os-mcp-runtime` 已把阶段 1-5 的主路径落到代码中：

- `macaca-framework::mcp` 是唯一真实 MCP protocol/tool adapter 入口，支持 stdio、streamable HTTP、SSE event-stream 响应解析、timeout、content fallback、namespace/collision policy。
- `macaca-web::mcp_runtime` 负责 OS 级 MCP definition、global config、app overlay、skill metadata discovery、policy filtering、runtime key/ref 计数、session/app/all cleanup。
- `framework_toolkit::build_toolkit` 是所有 traced agent 的统一 MCP 注入点，global/app/skill-discovered MCP tools 都经由同一路径进入 toolkit。
- lifecycle 事件统一写入 EventLog 并推送 live SSE：`mcp_server_resolved`、`mcp_server_starting`、`mcp_server_ready`、`mcp_tools_registered`、`mcp_server_failed`、`mcp_server_closed`。
- `skill_mcp.rs` 保留兼容 API，但不再作为主路径直接 spawn MCP；skill metadata 只作为 discovery source。

仍需单独演进的后续项：

- `macaca-mcp` crate 的最终废弃/薄封装属于 consolidation 阶段，不属于阶段 1-5 的主路径。
- HTTP MCP 当前覆盖 JSON-RPC over HTTP 与 SSE event-stream 响应解析；如果接入严格 legacy SSE 双通道服务器，需要在 `HttpMcpClient` 内继续扩展 session endpoint 发现。

建议第一步只实现：

- `macaca-framework` 新增 transport config 和 HTTP MCP client。
- `register_mcp_tools` 支持 namespace/collision policy。
- 保持现有 `skill_mcp.rs` 行为不变，只让它调用更完整的 framework MCP API。
- 增加 stdio + streamable HTTP 的测试。

这样可以先把协议层打稳，风险低，后续再把 OS registry/runtime 接上。

## 风险

- MCP 协议实现不完整会导致不同 MCP server 兼容性不一致。
- Stateful 服务如果按全局复用，会出现浏览器/IDE/terminal 状态串扰。
- 如果 lifecycle 资源释放依赖 toolkit drop，长 session 和异常断连场景仍可能泄漏进程。
- MCP 工具太多会污染 agent prompt，需要支持 tool namespace、policy 和按需暴露。
- `skill_mcp`、`macaca-mcp`、`framework mcp` 三套路径如果不统一，会继续产生行为差异。

## 结论

AgentScope 的设计验证了 MCP 应作为 toolkit-level primitive，而不是业务应用里的特殊工具。Macaca 应把 MCP 协议能力沉到 `macaca-framework`，把安装、策略、实例生命周期和观测放到 Agent OS 层。这样才能做到：安装一次 MCP 服务，所有 application 在统一权限和 trace 体系下使用。
