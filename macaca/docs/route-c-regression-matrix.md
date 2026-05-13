# Macaca OS 路线 C 回归矩阵

## 1. 目的

路线 C 会长期改造 Macaca 的微内核、系统服务、Application ABI、Plugin、Store、Web3/EVM。所有阶段都必须保护当前用户可见能力。本矩阵定义后续阶段必须持续验证的基线场景。

## 2. 回归场景总表

| ID | 场景 | 输入 | 观测点 | 预期结果 | 自动化状态 |
| --- | --- | --- | --- | --- | --- |
| RC-APP-001 | YAML application 加载 | 现有 app manifest | AppRuntime / Kernel agent count | app running，agent registered | 已有 integration tests |
| RC-CHAT-001 | `/api/chat/v2` 创建 session | 用户新消息 | session id、EventLog、SSE | session 创建，main thread 有事件 | 待 Web smoke 自动化 |
| RC-CHAT-002 | `/api/chat/v2` 恢复 session | 既有 session id + 新消息 | EventLog、resume signal | 正确恢复同一 session | 待 Web smoke 自动化 |
| RC-GOAL-001 | goal -> planner -> task -> worker -> review -> coordinator resume | create_goal | TodoBoard、RunTrace、EventLog | 任务完成后 coordinator resumed | no-network pipeline + live smoke |
| RC-TRACE-001 | trace 实时推送 | agent/tool/driver event | SSE stream | 不刷新即可看到事件 | 待 Web smoke 自动化 |
| RC-TRACE-002 | trace 历史恢复 | 刷新 session | EventLog replay | 历史 trace 完整且不重复 | 待 Web smoke 自动化 |
| RC-TASK-001 | task board session-scoped fetch | app_id + session_id | `/api/apps/{app_id}/todos?session_id=` | 只返回当前 session tasks | 已有/新增测试 |
| RC-DRIVER-001 | driver execution trace | driver tool call | delegate tab / EventLog | driver 名称和具体动作可见 | 待 driver smoke 自动化 |
| RC-SKILL-001 | skill/MCP runtime smoke path | skill-backed MCP tool | service/tool trace | skill/MCP 能调用并有 trace | 待 integration smoke 扩展 |
| RC-WASM-CERT-001 | WASM certification gate | dev/default/hardened certification profiles | sanitized report、reason codes、industrial-ready flag | hardened profile 通过才可标记 industrial-ready | runtime-host `wasm_certification` |
| RC-WASM-CERT-002 | WASM negative security fixtures | raw env/filesystem/network、missing trace/capability、oversized payload、timeout/resource exhaustion | fail-closed reason codes | 所有 negative case 在执行前拒绝且报告不泄露 raw payload/secret/env/API key | runtime-host `wasm_certification` |
| RC-WASM-PROVIDER-001 | hardened provider contract mock | trace、timeout、cancellation、backpressure、diagnostics envelope | provider-neutral response metadata | out-of-process profile 共享 WASM runtime provider 语义，不新增 application ABI | runtime-host `wasm_certification` |
| RC-WASM-OBS-001 | WASM runtime production telemetry | availability、session、invoke、host import、daemon、lifecycle、certification、supply-chain event | sanitized Observer sink events | sink 只接收 reason code / trace id / safe metadata，不记录 raw payload、secret、env、API key，且 sink 缺失不改变 runtime 结果 | runtime-host `wasm_telemetry` |
| RC-WASM-OBS-002 | WASM telemetry redaction | telemetry event with sensitive marker metadata | in-memory sink snapshot、tracing sink output | unsafe markers are redacted or dropped before sink storage | runtime-host `wasm_telemetry` |
| RC-RECOVERY-001 | frontend/backend 重启后 session 恢复 | 重启后打开 session | EventLog replay + live increment | 历史加载，增量继续推送 | 手工 + 后续自动化 |
| RC-PIPE-001 | 无网络 LLM pipeline dry run | scripted LLM | TaskSpace/TaskBoard/AgenticLoop | 全链路无外部依赖通过 | 阶段 0 自动化 |

## 3. 每阶段必须引用的最低基线

| 后续阶段 | 必须引用的回归场景 |
| --- | --- |
| 阶段 1 微内核边界 | RC-APP-001、RC-GOAL-001、RC-PIPE-001 |
| 阶段 2 系统服务 | RC-GOAL-001、RC-TRACE-001、RC-SKILL-001 |
| 阶段 3 Service Bus | RC-GOAL-001、RC-TRACE-001、RC-PIPE-001 |
| 阶段 4 Package Manifest | RC-APP-001、RC-CHAT-001 |
| 阶段 5 Application ABI | RC-APP-001、RC-CHAT-001、RC-GOAL-001 |
| 阶段 6 GenUI | RC-TRACE-001、RC-TRACE-002、RC-RECOVERY-001 |
| 阶段 7 Plugin Runtime | RC-DRIVER-001、RC-SKILL-001 |
| 阶段 8 Store / Entitlement | RC-APP-001、RC-SKILL-001、RC-TRACE-001 |
| 阶段 9 A2A Payment | RC-GOAL-001、RC-TRACE-001 |
| 阶段 10 Web3 | RC-APP-001、RC-TRACE-001 |
| 阶段 11 EVM | RC-APP-001、RC-TRACE-001 |
| 阶段 12 Web/CLI Thin Shell | RC-CHAT-001、RC-CHAT-002、RC-TRACE-001、RC-TRACE-002、RC-TASK-001 |
| 阶段 13 生态硬化 | 全部，且必须包含 RC-WASM-CERT-001、RC-WASM-CERT-002、RC-WASM-PROVIDER-001、RC-WASM-OBS-001、RC-WASM-OBS-002 |

## 4. 失败判定

以下情况一律视为回归：

- session trace 需要刷新才出现实时事件。
- 刷新后历史 trace 缺失或重复。
- task board 没有 session_id 时退化成全 app 扫描。
- worker task 完成后 coordinator 没有 resume。
- driver trace 只显示泛化名称，不显示 driver 名和动作。
- skill/MCP 调用没有 trace。
- Web3/EVM 缺失导致普通 application 失败。
- 任意阶段引入 application-specific hardcode。

## 5. 阶段 0 自动化基线

阶段 0 必须提供 `route_c_baseline` 集成测试：

- 校验本矩阵中关键场景名称存在。
- 执行现有 no-network pipeline dry run。
- 不依赖真实 LLM、浏览器、前端服务器或外部网络。
