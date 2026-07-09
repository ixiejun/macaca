# Macaca OS 宪法合规性全面审计报告

- **日期**: 2026-07-08
- **依据**: 三部开发宪法 — `macaca-os-architecture-governance.md`、`macaca-os-microkernel-boundaries.md`、`macaca-os-serviceization-allowlist.md`
- **范围**: 全部 27 个 workspace crate（kernel / foundation / services×12 / runtime×4 / application / facade / shells / packages / tests），含未提交的 domain_pack 变更
- **方法**: 五路并行专项审计（内核与基础层、服务层、SDK 与应用框架层、Shell 与 runtime-host 层、门禁与横向 bug 扫描），全部发现均附文件:行号真实代码证据；`cargo check --workspace` 通过；文件大小硬门禁实测**失败**。

---

## 一、总体结论

架构骨架健康：依赖方向纪律良好（kernel 仅依赖 proto+ipc；SDK 仅依赖 proto；shell 收敛到 proto+SDK，唯 CLI 例外）；trace-required 中间件、脱敏 hash、Null Object unavailable、组合根 provider 构造等宪法机制大体落实；六类宪法门禁均有可执行测试。

但存在四类系统性债务：

1. **【最严重】生产副作用点无门禁**：skill/tools 的进程执行与任意路径读写、task 全部写命令，在副作用前完全没有 policy/entitlement/资源/预算检查；多处安全判定 fail-open。
2. **业务语义下沉越界**：foundation proto 承载 12 个业务域约 127 文件 43k 行契约（含审批分类、报表语义）；payment 契约进入 foundation persist；Web3 类型进入 foundation ipc；shell（macaca-web）仍拥有 prompt 构造、重试、fallback 规划、Task Board 修复等系统语义；CLI 绕过 SDK 直连后端。
3. **provider/模型名硬编码**：llm 定价表按模型名分支、按 provider 名 match 构造、minimax URL 写死；proto 默认配置写死 DashScope/Milvus/Telegram；framework 内 DashScopeFormatter；skill/provisioner 甚至用 `concat!` 拆字面量规避检测。
4. **正确性 bug 面**：3 处 UTF-8 字节切片 panic、任务重复 claim 竞态、调度器重试退避失效与 run-id 字典序错乱、心跳把在途 run 覆写、事件日志谎报持久化、上下文截断产生 orphaned tool message（API 400）、service_router 对非幂等操作盲目重试（可重复支付/部署）等。

**门禁即时风险**：未提交的 `ai_common.rs`（526 行）已导致 `os_layer_file_size_gate` 实测 FAILED —— **本批变更提交前必须先拆分该文件**。

---

## 二、P0 — 安全与崩溃级问题（必须立即修复）

### P0-1 技能/工具执行链完全无门禁
- `crates/services/macaca-skill/src/tool.rs:52-113` + `adapter.rs:22-37`：`execute_shell_entry` 直接 `Command::new` 拉进程，command/args/work_dir 来自 YAML 与 LLM 输入原样拼接，无 policy/entitlement/预算/资源检查，不携带 TraceContext。
- `crates/services/macaca-tools/src/builtin.rs:210-247`：`ShellTool::invoke` 直接 `sh -c` 执行任意命令，无任何门禁；stdout/stderr 无界读入。
- `crates/services/macaca-tools/src/builtin.rs:77-95,130-167`：FileRead/FileWrite 任意路径读写，无沙箱。
- 违反：allowlist「副作用前过 policy/资源/预算/entitlement」「无 policy 不得调用能力」。
- **修复**：所有进程执行/文件读写下沉 runtime-host provider 装饰链；spawn 前强制 TraceContext + policy 决策 + 配额；work_dir/路径做工作区根白名单校验，越界返回结构化 denied；输出设字节上限并标注 truncated；超时后 `child.kill()`（当前 tool.rs:88-96 超时子进程泄漏）。

### P0-2 技能路径穿越绕过（fail-open）
- `crates/services/macaca-skill/src/runtime/path_policy.rs:11-26`：`canonicalize` 失败时回退原始路径再做 `starts_with` 前缀比较（不解析 `..`），`/<base>/../../etc/passwd` 可逃逸 skill 目录。
- **修复**：canonicalize 失败一律返回 false；或先做词法归一化并拒绝含 `..` 组件。参照同 crate `projection.rs` 的 `symlink_metadata` 正确范式。

### P0-3 策略/证据门禁 fail-open（应 fail-closed）
- `crates/services/macaca-skill/src/evolution.rs:48-52,83-86,137-139`：证据网关 `default()` 返回 Accepted、`verified_terminal_success` 默认 true，JSON 缺字段即绕过证据门禁。
- `crates/services/macaca-skill/src/proposal_lifecycle.rs:175-177`、`proposal_processing.rs:183-187`、`curation.rs:82-104`：readiness 为 `None`（未知）时放行 Promote/Reject/apply 副作用，与 `mutation.rs:167` 的 `==Some(true)` 标准不一致。
- `crates/services/macaca-autonomy-evolution/src/live_orchestrator.rs:305-337`：文档称"校验 lease 与幂等证据"，实际仅 `trim().is_empty()` 判空。
- **修复**：所有 default 改 Missing/Rejected/false；副作用统一要求 `entitlement_ready==Some(true) && package_ready==Some(true)`；补真实 lease 有效性/所有权/过期校验。

### P0-4 UTF-8 字节切片 panic（3 处，中文场景必崩）
- `crates/kernel/macaca-kernel/src/logging.rs:88,99,107,134`：`&text[..12]`/`[..40]`/`[..max_len]` 等字节切片，`log_tool_call → truncate(500)` 对中文工具参数必然 panic；同时 `mask_sensitive` 保留 sk- 密钥前 12 字符 / Bearer 前 8 字符进日志（泄漏）。
- `crates/services/macaca-task/src/decompose.rs:134-140`：错误分支 `&json_str[..min(500)]`（147 行 `chars().take(500)` 才正确）。
- `crates/services/macaca-gateway/src/telegram_format.rs:12,25`：`split_message` 用 `&remaining[..max_len]` 字节切分，中文/emoji 消息发送崩溃。
- **修复**：统一 `char_indices()`/`floor_char_boundary` 找安全边界；密钥整体遮蔽不保留前缀。建议在 foundation 提供共享的 `safe_truncate` 工具并全仓替换。

### P0-5 虚假脱敏（比不脱敏更危险）
- `crates/services/macaca-autonomy-evolution/src/governance_ledger.rs:416-428` / `os_code_proposal_adapter.rs:323-335`：`sanitize_ref` 仅替换字面词 secret/credential，真实密钥值（sk-…、Bearer …）原样落盘。
- `crates/services/macaca-autonomy-evolution/src/local_provider.rs:291-360`：内存 ledger 路径完全绕过脱敏（JSONL 路径走、内存路径不走）。
- **修复**：脱敏统一在 DTO 构造/契约边界执行，改结构白名单（受控标识符/URI 格式，不符值拒绝或哈希化），所有 provider 共享同一净化结果。

### P0-6 门禁破坏：ai_common.rs 超 500 行硬限
- `crates/foundation/macaca-proto/src/domain_pack_contract/ai_common.rs`：526 行全生产契约，`cargo test -p macaca-integration-tests --test os_layer_file_size_gate` 实测 FAILED（gate.rs:218 panic）。
- **修复**：按稳定归属拆分为 `ai_common_model.rs` / `ai_common_commands.rs` / `ai_common_hashes.rs`，对齐同目录 `finance_accounting_*` 的既有拆分方式。**未提交变更 commit 前必须完成。**

---

## 三、P1 — 宪法边界违规

### 3.1 内核与 foundation 层

| # | 级别 | 位置 | 问题 | 修复 |
|---|------|------|------|------|
| K1 | 高 | `macaca-persist/src/event_log.rs:178-203` | `append_command` 文档声称立即持久化，实际 `let _ = set(...)` 吞写入错误且无条件返回 seq；`next_seq`(143) 先自增，写失败产生重放序列空洞 | 返回 `MacacaResult<u64>`，失败传播或回退计数器并记 audit；索引写入(130-132)同理 |
| K2 | 高/中 | `macaca-proto/src/config/root.rs:71-109` | `RootConfig::default()` 硬编码 milvus/localhost:19530、text-embedding-v4、DASHSCOPE_API_KEY、dashscope URL、Telegram/Discord | Default 只给中性空值，provider 具体值移应用层 `config/default.toml` |
| K3 | 中 | `macaca-ipc/src/web3_bridge.rs` + `lib.rs:16,31` | Web3 可选模块类型进入 foundation IPC，内核编译闭包被迫包含 Web3 | 下沉可选 crate 或 `#[cfg(feature="web3")]` 门控 |
| K4 | 中 | `macaca-ipc/Cargo.toml:15` | `async-nats` 非 optional，具体传输 provider 成基座强依赖 | `optional = true` + `nats` feature |
| K5 | 中 | `macaca-kernel/src/alert.rs:123-160` | `all_llm_degraded`/`budget_warning`/`worker_restart_warning` 把 LLM 降级链/成本/worker 语义写进内核 alert 原语 | 便捷方法移对应服务层，内核只留 `fire(Alert)` |
| K6 | 中 | `macaca-kernel/src/execution_port.rs:52-55` | 读锁 guard 跨整个 agent 执行 await，`replace()` 热替换被长执行阻塞 | clone 后 drop guard 再 await |
| K7 | 中 | `macaca-persist/src/payment_store.rs`（lib.rs:20 导出） | 支付领域契约（quote/receipt/intent/状态迁移）位于 foundation persist | 移入 payment 服务 crate，foundation 只留中性 KV/Memento 原语 |
| K8 | 中 | `macaca-proto/src/domain_pack_contract/`（127 文件 ~43k 行） | 12 个业务域契约编入基座 proto（见 3.3） | 拆独立 `macaca-domain-pack-contracts` crate（见第六章方案 R2） |

合规确认：kernel 依赖方向合规、无静态状态违规、生产路径无 unwrap/expect、trace 门控落实（`TraceRequiredMiddleware`）、文件全 <470 行（audit.rs 469、logging.rs 449 需关注）。

### 3.2 服务层（12 crate）

**provider/模型名硬编码（Rejection List「OS 层不得按 provider/model 名分支」）**

| # | 级别 | 位置 | 问题 | 修复 |
|---|------|------|------|------|
| S1 | 高 | `macaca-llm/src/cost.rs:20-59` | `default_pricing()` 按模型名分支硬编码 gpt-4o/claude-3-opus 等定价表 | 定价改配置注入（参照 resolver.rs descriptor 数据表范式） |
| S2 | 高 | `macaca-llm/src/coding_plans.rs:12-24` | 按 `provider_name=="minimax"` 分支写死 `https://api.minimaxi.com/v1` | base_url 规范化纯数据化，厂商修正作 provider 配置项 |
| S3 | 中 | `macaca-llm/src/router.rs:125-130` | `from_config` 按 provider 名 match 直接 new 具体 provider（非批准组合根） | Abstract Factory + 可替换注册表，构造移 runtime-host 组合根 |
| S4 | 中 | `macaca-skill/src/provisioner.rs:15-59`、`source.rs:65`、`discovery.rs:226-237` | 硬编码 client 名单（claude/cursor/gemini-cli/codex…），且用 `concat!` 拆字面量规避门禁检测 | client 表由配置/persona 注入；此规避行为应在 review 规范中点名禁止 |
| S5 | 中 | `macaca-memory/src/embedding.rs:63-115` | DashScopeEmbedding 以厂商名直接顶层导出+默认厂商 URL | 经 EmbeddingProviderFactory 由组合根注册 |
| S6 | 低 | `macaca-task/src/decompose.rs:219,250,262-304` | 硬编码 "plan_agent"/"entry_agent" 角色名与内嵌 planner 提示词（含 "backend, or frontend"） | 角色由 contract 传入，提示词模板可注入 |

**原始 payload / 无界输出进观测面（Security Rules）**

| # | 级别 | 位置 | 问题 | 修复 |
|---|------|------|------|------|
| S7 | 高 | `macaca-llm` anthropic.rs:231-237 / dashscope.rs:233-239 / openai.rs:227 / openai_compatible.rs:252 | 非 2xx 时原始响应体未脱敏、无上界拼入错误串并进 tracing | 复用 macaca-memory 的 `redact_text`+截断范式，llm crate 目前完全缺失 |
| S8 | 高 | `macaca-memory` embedding.rs:160-165,200-206、vector.rs:136,214,246,291 | embedding 与 Milvus 错误路径原始响应体入错误串（同 crate remote.rs 已正确用 redact_text，不一致） | 统一走 redact_text+截断 |
| S9 | 高 | `macaca-gateway/src/telegram.rs:114`、gateway.rs:79-124 | `warn!(body=%body)` 打印完整 API payload；DefaultEventHandler info 级打印完整用户消息 | 只记录结构化 error_code/description；content 截断脱敏 |
| S10 | 中 | `macaca-tools/src/tool.rs:184-225`、builtin.rs:135 | trace 中间件把工具 input/output 原样全量写入 trace；`error!(raw_input=%input)` | input/output 脱敏+截断或只记摘要/哈希 |
| S11 | 高 | `macaca-skill/src/tool.rs:98-112` | 子进程输出无界读入，完整命令行（可能含密钥）回写结果 | 输出上限+命令行脱敏 |

**假成功 / 静默降级（「absence 必须结构化，crash/hang/silent fallback/fake success 均非法」）**

| # | 级别 | 位置 | 问题 | 修复 |
|---|------|------|------|------|
| S12 | 高 | `macaca-gateway/src/telegram.rs:67-77,169-179` | token 缺失时 start/send_message 仅 warn 后 `Ok(())`，回复被静默丢弃并报成功 | 返回结构化 unavailable |
| S13 | 高 | `macaca-gateway/src/discord.rs:44-65` + builder.rs:62-66 | Discord 是纯 stub 却在 enabled=true 时真实注册，生产静默丢消息 | stub 返回结构化 unsupported，或 builder 拒绝注册 |
| S14 | 高 | `macaca-tools/src/orchestration.rs:303-310` | ListAgents 无 provider 时静默返回空数组冒充成功 | callback None 返回结构化 unavailable |
| S15 | 高 | `macaca-scheduled-agent-task/src/local_provider.rs:161-199` | create_task 先写 state+payload 再注册 job，注册失败不回滚，留下永久 active 僵尸任务 | 失败路径回滚 state.tasks 与 payload |
| S16 | 中 | `macaca-task/src/plan_loop/goal_evaluator.rs:66-108` | 评估 JSON 解析失败返回 `Satisfied{"parsing fallback"}`，无法判断被当作已满足 | 解析失败返回 NeedsMoreWork 或结构化 failure |
| S17 | 中 | `macaca-driver/src/loader.rs:141-144` | config 序列化失败静默以 `{}` 空配置"成功"加载驱动 | 返回结构化 Driver 加载失败 |
| S18 | 中 | `macaca-skill/src/facade.rs:139-148` | skill 加载失败仅 warn+continue，契约已有 failures 字段却全丢弃 | 返回结构化 loaded/failed/failures |

**副作用前无 policy / trace 缺失**

| # | 级别 | 位置 | 问题 | 修复 |
|---|------|------|------|------|
| S19 | 高 | `macaca-task/src/runtime/` task_lifecycle_commands.rs:20-266 / assignment_commands.rs:19-87 / goal_commands.rs:25-63 | 所有写命令仅非空校验即改 store；descriptor 声明 `task.manage` 权限但无处强制 | 副作用前统一 policy 校验钩子，拒绝返回结构化 denied |
| S20 | 中 | `macaca-llm/src/router.rs:140-152` + resilient.rs:179-188 | from_config 置 `max_budget_usd:None`，预算 gate 事实被绕过；60s×3 退避期间占用限流槽 | 从 LlmConfig 注入预算；复核退避占槽 |
| S21 | 中 | `macaca-llm/src/provider.rs:14-18`、`macaca-tools/src/tool.rs:61-70` | provider 端口/工具上下文 trace 全 Option，可绕过装饰器直调（`impl LlmProvider for LlmRouter` 使 router 可被当 provider 直调） | 保证一切入口经 runtime-host 装饰器，或端口签名强制 trace；需专项确认 |
| S22 | 中 | `macaca-gateway/src/telegram.rs:89-160` | 全链路无 trace 传播，send 前无 policy/预算检查 | 事件契约加 trace 贯穿，出入站插 policy 检查点 |
| S23 | 中 | `macaca-skill/src/service_contract.rs:167-185`、operator_lifecycle.rs:122-139 | 多数命令可空 trace 构造；最高风险的 InvokeCommand 无 trace/scope/policy 字段 | 全部命令强制 validate_trace 构造函数 |
| S24 | 中 | `macaca-scheduler` service.rs:209-274,315-347 | trigger_job 无 scope 校验（任意调用方触发任意 scope job）；get_run 不按 job scope 过滤，跨 scope 泄漏 | trigger 补 scope 校验；run 查询按所属 job scope 过滤 |
| S25 | 中 | `macaca-context/src/service_contract.rs:131-142` | `into_engine_input()` 丢弃 policy（privacy_tier 等），策略可能被静默忽略 | into_engine_input 承载 policy 或契约明确 provider 显式消费+测试断言 |

**Autonomy 语义泄漏进工具层（严重级）**

| # | 级别 | 位置 | 问题 | 修复 |
|---|------|------|------|------|
| S26 | 严重 | `macaca-tools/src/todo/create_todo.rs:37-88,154-196` | 工具层硬编码多语言关键字→能力映射（"架构"→architecture、"前端"→ui）、停用词表、foundation 角色分类、自动跨任务依赖推断 —— 任务分解/角色决策属 Task/Autonomy 服务 | 抽成 macaca-task 提供的 `DependencyInferenceStrategy` trait 注入，工具层仅 DTO 透传 |
| S27 | 中 | `macaca-autonomy-evolution/src/admission.rs:136-235` | admission 门禁硬编码 Skill 包专有语义（"skill-exp-" 前缀、"skill.md"），换应用即失效 | Skill 特定判定下沉可替换 provider |

### 3.3 SDK / 应用框架 / domain-pack 层

| # | 级别 | 位置 | 问题 | 修复 |
|---|------|------|------|------|
| A1 | 高/中 | `macaca-proto/src/domain_pack_contract/`：finance_accounting_preflight.rs:6-42、finance_accounting_bounds/_reports、finance_crypto.rs:12-70 等 127 文件 | READ/PLANNING/APPROVAL/WRITE/REPORT 审批分类、试算表/资产负债表/损益/现金流报表语义、bounds/preflight 校验属具体业务规则，下沉到了比应用框架更低的 foundation proto | proto 只留中性 DTO/命令名/错误类型；审批分类/bounds/reports 上移 domain pack 或 Task/Autonomy 服务；整体拆独立 contracts crate（方案 R2） |
| A2 | 中 | `industrial_reference_catalogs.rs:128-149`、`industrial_pack_taxonomy.rs:143-248` | `("finance","crypto")=>Some(...)` 领域名长 match 硬编码注册表 | 数据驱动目录，pack 在组合根自注册，proto 只持 trait+聚合器 |
| A3 | 中 | `macaca-framework/src/formatter.rs:357-400` | DashScopeFormatter + Qwen 原生响应解析硬编码进通用 agent 框架 | 移 LLM provider 适配层，框架只依赖 Formatter trait |
| A4 | 中 | `macaca-app/Cargo.toml` + llm_proxy.rs:9-49 + skills.rs:5 | 应用框架直接依赖 macaca-llm/tools/skill/kernel，持有 LlmProxy 与 skill 解析，越出 manifest/ABI/生命周期契约边界 | 改经 SDK/服务边界访问，或 OpenSpec 明确记录其中介边界 |
| A5 | 低 | `macaca-sdk/src/domain_pack_accounting_client.rs` vs `domain_pack_client.rs` | 仅 accounting 有专属 builder，骨架不一致，按域线性膨胀风险 | 抽泛型 domain preflight builder 骨架；定"仅高风险写入域需专属 client"准则 |
| A6 | 低 | `macaca-sdk/src/domain_pack_bridge.rs`（~20KB） | 数百符号手工 pub use，新增 pack 易漏同步 | 分模块整体转发或受控 prelude |

合规确认：SDK 仅依赖 proto、不构造任何 provider/runtime/wallet；macaca-domain-pack-finance 承载 binance/okx URL 位置正确；SDK 生产路径无裸 unwrap；`domain_pack_accounting_client.rs` 本身干净（preflight+委托，无业务逻辑）。

### 3.4 Shell 层 / runtime-host

| # | 级别 | 位置 | 问题 | 修复 |
|---|------|------|------|------|
| W1 | 严重 | `macaca-web/src/loop_manager/worker_loop_orchestrator.rs:83-102` | shell 内拼 "Execute this task:…" 执行 prompt（含验收标准/上下文） | 下沉 Task/AgentExecution service 命令，shell 只传结构化字段 |
| W2 | 严重 | 同上 :213-216 | shell 内拼 "Retry task:…Feedback:…" 重试 prompt | RetryTask 载荷结构化，prompt 由 service 生成 |
| W3 | 严重 | `plan_event_goal_lifecycle.rs:226-230` | shell 拼 replan prompt（"needs additional work…create_todo"），绕过既有 service prompt 构建 | 补 `build_followup_planning_prompt` service 命令 |
| W4 | 严重 | `macaca-cli/Cargo.toml:27` + skill_operations/live_client.rs:76-83、tool_operations.rs:91-95、workbench_operations.rs:136 | CLI 用 reqwest 自建 3 组 HTTP 客户端直连 3001，硬编码 REST 路由拓扑/错误协议/payload 契约 | 全部下沉 SDK 客户端，移除 reqwest 依赖 |
| W5 | 高 | `plan_event_goal_lifecycle.rs:120-134,281-303` | prompt 构建/解析失败时 shell 以 `_fallback` 直接把 goal 标记完成（"marking complete by default"）——终态修复规则在 shell 且是假成功 | service 返回显式 outcome，shell 仅渲染 |
| W6 | 高 | `loop_manager/decomposition_adapter.rs:78-118` | shell 自判 Pending/Blocked/Assigned 应取消并 `todo_store.save_todo` 直写持久层 | 新增 `cancel_partial_goal_tasks` service 命令 |
| W7 | 高 | `macaca-runtime-host/src/service_router.rs:177-220` | 统一 router 对所有失败/超时一律重试，不查幂等性；evm/payment/gateway 非幂等写操作会重复副作用（重复部署/支付） | 重试前从 descriptor 读 idempotent 标志，仅幂等操作重试 |
| W8 | 中 | `worker_loop_orchestrator.rs:36-41` | "entry/plan agent 不从 TaskBoard 拉任务"角色规则硬编码在 shell | 由 agent capability 声明决定 |
| W9 | 中 | cli tool_operations.rs:38 等 3 处 | 硬编码 "http://127.0.0.1:3001" | 从 config/SDK 读取 |
| W10 | 低 | `framework_runner/context_prompt_builder.rs:59,78,102-113` | shell 硬编码 fallback system prompt | 模板由 context/app service 提供 |

合规确认：web/cli 依赖已收敛 proto+SDK（除 CLI reqwest）；service_router 落实 policy 先行/trace-required/payload hash 脱敏；provider 构造均在组合根；无 provider 名路由分支。

### 3.5 门禁体系自身

| # | 级别 | 位置 | 问题 | 修复 |
|---|------|------|------|------|
| G1 | 严重 | `os_layer_file_size_gate` | 实测 FAILED（ai_common.rs 526 行），见 P0-6 | 拆分后恢复绿灯 |
| G2 | 中 | `sdk_no_provider_construction_gate.rs:43-86,112` | 硬编码 11 token denylist 行内子串匹配：新 provider 漏检、可被别名/换行绕过、注释误报（skill/provisioner 的 `concat!` 拆字面量即是实证） | 改 provider 命名模式正则（`*ServiceProvider::`、`*Provider::new`）+ 新 provider 强制登记元测试 |
| G3 | 低 | `protocol_service_dependency_boundaries/gate.rs:362` | 基于 Cargo.toml 依赖图，无法发现允许依赖内部的越权 use，盲区靠不完整 token 门禁补 | 增加 use 语句级 AST 扫描门禁 |

---

## 四、P2 — 正确性 bug 与设计缺陷

### 4.1 状态机与并发

| # | 级别 | 位置 | 问题 | 修复 |
|---|------|------|------|------|
| B1 | 高 | `macaca-heartbeat/src/command_handler.rs:53-87` | wake coalesce 无条件把已 Running 的在途 run 覆写为 Coalesced 且返回 accepted=true | coalesce 前判断目标非终态且非 Running，对 Running 返回 Busy/Conflict |
| B2 | 高 | `macaca-scheduler/src/local_provider/run_control.rs:353-376` | next_lease_candidate 不检查 `scheduled_for<=now`，重试 run 被立即派发，退避完全失效（重试风暴） | 过滤加 `scheduled_for<=now` |
| B3 | 高 | `macaca-scheduler/src/local_provider/store.rs:65-69` | `run-{seq}` 字典序 BTreeMap，"run-10"<"run-2"，两位数后 FIFO/最近 N/租用顺序全部错乱 | 零填充 `run-{:020}` 或数值键 |
| B4 | 高 | `macaca-autonomy-evolution/src/local_provider.rs:235-289` | live tick 幂等检查 TOCTOU：检查/自增/写入分三次取锁，并发重复副作用+序号双增 | 合并单锁作用域或 entry API 原子占位 |
| B5 | 中 | `macaca-task/src/todo_board/task_board.rs:66-142,153-193` | claim 是 read→modify→save 非原子，无 CAS/租约，并发重复 claim 同一任务 | save 带版本号条件写或 per-(app,session) 串行化 |
| B6 | 中 | task/scheduler/heartbeat 各 transition（如 run_control.rs:169-268、run_lifecycle.rs:202-241） | 状态转换只判存在不判当前态：终态可重复转移、Paused job 可被 trigger、幽灵重试 | 引入合法迁移矩阵，非法迁移返回结构化 Conflict/InvalidRequest |
| B7 | 中 | `macaca-task/src/todo_store.rs:203-212` | 崩溃恢复只回滚 TodoItem，TodoGoal 停在 Decomposing/Evaluating 非终态即永久丢失 | 补 goal 崩溃恢复回滚 Pending |
| B8 | 中 | `macaca-runtime-host/src/skill_service_provider_merge.rs:174-200` | 单语句并持 6 把 tokio 锁跨 await，取锁顺序不同即死锁 | 先克隆局部释放锁再构造 |
| B9 | 中 | `macaca-task/src/worker_loop.rs:122-138,184-195` | claim 后事件 send 吞错 + wait_for_task_exit 无超时轮询，worker 可无限期卡住 | send 失败回滚状态并 break；等待加超时上限 |
| B10 | 中 | `macaca-runtime-host/src/executor/queue.rs:312,335` | complete()/fail() `let _ = tx.send(result)` 静默丢任务结果（对应 CLAUDE.md 已知坑"Worker 完成但状态不更新"） | send 失败至少 warn 并回写状态 |
| B11 | 中 | `macaca-scheduler/src/local_provider/schedule.rs:196-204,214-239` | cron DOM/DOW 用 AND（标准 Vixie 为 OR）漏触发；Step 忽略 min 偏移、不支持范围/列表 | 实现标准语义或解析期拒绝不支持形态 |
| B12 | 中 | `materialization.rs:48-52` + schedule.rs:72-93 | stagger delay 被累加进 last_scheduled_at 锚点，固定间隔逐轮漂移 | 锚点记录未 stagger 的真实 due |
| B13 | 中 | `macaca-driver/src/sdk.rs:195,384,412` + dynamic_proxy.rs:152-164 | SDK 宏每次 FFI `block_on` 新建 runtime，宿主 health_check/shutdown 未走 spawn_blocking → "Cannot start a runtime from within a runtime" panic 被吞，动态驱动健康检查恒失败 | 宿主侧统一 spawn_blocking；或 SDK 检测 Handle::try_current() 复用 |
| B14 | 中 | `macaca-driver/src/dynamic_proxy.rs:134-135` | 流式回调 user_data 指向栈局部对象，插件延迟回调即悬垂指针 UAF | ABI 明确回调须同步完成，或 Arc+join 护栏 |
| B15 | 中 | `macaca-runtime/src/context_window.rs:105-123` + agentic_loop/iteration.rs:132-138 | 截断边界可落在 assistant(tool_calls) 与 tool 结果之间，orphaned tool message 导致 OpenAI/DashScope 400 | 截断边界回退到 System/User 边界或成对保留；另 :81-83 截断后不复检 token 无二次收敛 |
| B16 | 中 | `macaca-gateway/src/telegram.rs:111-117` | getUpdates 无 result 时无 sleep 直接 continue，409/限流场景 CPU 满载忙循环 | 该分支加退避，按 error_code 分类处理 |

### 4.2 锁毒化与资源

| # | 级别 | 位置 | 问题 | 修复 |
|---|------|------|------|------|
| B17 | 中 | `macaca-runtime-host/src/execution_control_runtime.rs`（8 处）、autonomy governance_ledger、scheduled-agent-task local_provider.rs:74-89、heartbeat memento.rs:28-42、task runtime/snapshot.rs 等、llm cost.rs（7 处） | 全仓对 std 锁毒化一律 `.expect(...)` panic，一次 panic 级联击穿服务（execution_control 甚至击穿崩溃恢复子系统自身）。生产 expect 共 191 处 | 统一 `unwrap_or_else(|e| e.into_inner())` 或 parking_lot；作为一次横向机械化整改 |
| B18 | 中 | task runtime/snapshot.rs:42-75、scheduler run_control.rs:378-417、scheduled-agent-task payload_store.rs:66-74、heartbeat metadata、autonomy diagnostics | 7×24 无界增长家族：快照 map 只增不淘汰、终态 run 不清理、payload 无 remove、诊断无上限 | 统一加 LRU/TTL/保留上限；扫描改索引 |
| B19 | 低 | unbounded channel 7 处（runtime/execution.rs:57、driver/session.rs:38、tools/tool.rs:360、web/framework_adapter.rs:172 等） | 生产者快于消费者时内存无界 | 评估 bounded+背压 |
| B20 | 低 | `macaca-runtime/src/loop_detector.rs:44,75-78` | recent_hashes/window_size 死代码，滑动窗口检测无效 | 删死字段或落实窗口检测 |
| B21 | 低 | `macaca-task/src/tracker.rs` | TaskTracker 与 TodoStore/TaskBoard 两套状态机并存，前者未被 runtime 引用 | 确认后移除或合并 |
| B22 | 低 | `macaca-task/src/runtime/graph_admission.rs:70-76` | 已存在权威任务 graph_id=None 时 or_else 自等回退，冲突判定永假，可放行第二权威 graph | None 视为占用，不做自等回退 |
| B23 | 低 | `macaca-runtime-host/src/service_router.rs:366-372` | 审计 hash 用 DefaultHasher(SipHash64) 非加密哈希 | 改 SHA-256 或注明非安全用途 |
| B24 | 低 | 吞错点：web sse.rs:45、goal_lifecycle_shell_adapter.rs:157、session_persistence_adapter.rs:55,64、host-composition application_agent_delegate_bridge.rs:111（delegate 回执丢失→调用方永久等待）、heartbeat command_handler.rs:184（失败仍耗审计序号） | 静默吞 Result | 至少 warn；delegate 回执丢失需回写失败状态 |

### 4.3 服务契约完整性

- `macaca-tools` 全 crate 无 service_id/descriptor/生命周期/健康检查/快照，命令结果均裸 `serde_json::Value`（S-高）。→ 引入 ServiceContract/ToolDescriptor + 类型化 DTO + 四类错误枚举。
- `macaca-gateway/src/service_adapter.rs:13-28`：descriptor 静态、health 恒 Healthy、`stop()` 空操作、后台轮询无 CancellationToken 无法停止（生命周期不完整，高）。
- `macaca-task/src/runtime/mod.rs:70-78`：全部 `Result<_, String>` 无结构化错误分类。
- `macaca-autonomy-evolution`、`macaca-skill`：缺 start/pause/resume/shutdown 生命周期算子；scheduled-agent-task health 恒 healthy 不探测 scheduler 依赖。
- `macaca-skill/src/parser.rs:40-58`：YAML 别名炸弹无防护；`agent_skill/model.rs:52-73` 公有入口无界文件读取；`provisioner.rs:280-304` 递归复制跟随 symlink 无深度上限（环→无限递归）。
- `macaca-skill/src/governance_store.rs:251-274`：审计事件 policy_decision_ids 硬编码空且无填充入口，审计链断裂。

---

## 五、文件规模（500 行宪法）趋势

- 超硬限（>500）：1 个 —— `ai_common.rs`（526，P0-6）。
- 450–500 预警区约 40 个生产文件，重点：
  - domain_pack_contract/ 8 个（model.rs 489、media_transcription.rs 487、finance_crypto.rs 487、foundation_filesystem.rs 486、finance_stock.rs 484…）
  - runtime-host：evm_service_provider.rs 499（单文件混 strategy+unavailable+mock+dispatch，优先拆）、application_execution_service_provider.rs 498、execution_control_session_loop.rs 497、config_service_commands.rs 497
  - web：alias_resolution.rs 490、application_execution_agent_event_bridge.rs 488
  - scheduler run_control.rs 454；context report.rs 487、memory.rs 465；skill evolution.rs 465、service_contract.rs 447（承载 14 类能力，上帝契约苗头）
  - kernel audit.rs 469、logging.rs 449

建议：按宪法「descriptor/dispatch/state-machine/adapter/support/tests」归属拆分，优先 evm_service_provider 与 domain_pack 近限文件（微调即越界）。

---

## 六、重构方案（更优雅的设计）

### R1 统一「副作用门禁装饰器」骨架
现状：policy/entitlement/trace 检查散落且多处缺失（S19、S22、P0-1、P0-3）。方案：在 runtime-host 已有 service_router 装饰链基础上，抽出可复用的 `SideEffectGuard`（trace 必填 → policy 决策 → entitlement/预算 → 资源配额 → 执行 → 审计回写），所有服务 provider 的写命令强制经过；工具/技能执行改为经该装饰链的 runtime-host provider。同时把「fail-closed」写成共享类型（readiness 必须 `Some(true)`）。

### R2 domain_pack_contract 拆出独立 crate
现状：127 文件 43k 行业务域契约在 foundation proto（K8/A1/A2），且 8 个文件贴近 500 行。方案：
1. 新建 `crates/foundation/macaca-domain-pack-contracts`（或按域再分 finance/commerce/…），macaca-proto 只保留 provider 中立的 domain pack 元框架（trait、聚合器、通用 DTO）。
2. 审批分类（preflight）、bounds、reports 语义上移到 domain pack 包或 Task/Autonomy 服务。
3. `industrial_reference_catalogs` 的领域名 match 改为 pack 在组合根注册的数据驱动目录。
4. SDK 侧抽泛型 preflight builder 骨架，12 个 `domain_pack_client_*_tests.rs`（1544 行雷同）改表驱动（可省约 1000 行）。

### R3 共享「脱敏与截断」基础设施
现状：memory 有 redact_text、autonomy 有 6 份重复 sanitize、llm/gateway/tools/kernel logging 各自缺失或错误（P0-4/P0-5/S7-S11）。方案：foundation 层提供 `sanitize` 模块（redact_text、safe_truncate（char 安全）、bounded_refs、metadata 白名单），全仓替换；门禁加一条「错误串禁止直接拼接 `.text().await`」的 AST 检查。

### R4 状态机迁移矩阵通用件
task/scheduler/heartbeat 三处同型缺陷（B6），抽 `TransitionMatrix<S>` 小工具（声明合法迁移，非法返回结构化 Conflict），三个服务复用，并补崩溃恢复回滚规则（B7）。

### R5 Shell 语义收编
W1–W6 的 prompt 构造/重试/replan/终态修复/直写持久层，统一收编为 Task/Autonomy 服务的 4 个新命令：`build_task_execution_prompt`（或直接结构化执行命令）、`retry_task`、`build_followup_planning_prompt`、`cancel_partial_goal_tasks`；goal 评估失败返回显式 outcome。CLI 的 3 组 HTTP 客户端下沉 SDK。

### R6 LLM provider 工厂化
S1–S3、S20：定价表与模型路由默认值全部 descriptor 数据化（resolver.rs 已是正确范式）；`from_config` 的按名 match 改 ProviderFactory 注册表并移入 host-composition；预算 gate 从配置强制注入。

### R7 锁毒化与无界增长横向整改
B17/B18 为机械化整改：全仓 `lock().expect("poisoned")` → `into_inner()` 恢复（或 parking_lot）；所有内存 map/Vec 增加保留策略。可各用一个专项 PR 完成。

---

## 七、执行计划（分阶段步骤）

> 每阶段完成后必跑：`cargo check --workspace`、`cargo test -p macaca-integration-tests`（全部门禁）、相关 crate 单测；OS 层行为变更按宪法走 OpenSpec 提案 + GitNexus impact 分析。

### 阶段 0：解除提交阻塞（0.5 天）
1. 拆分 `ai_common.rs`（526→3 个文件，对齐 finance_accounting_* 方式）。
2. 复跑 `cargo test --test os_layer_file_size_gate` 确认绿灯，随本批 domain_pack 变更一起提交。

### 阶段 1：P0 安全与崩溃修复（2–3 天）
1. P0-4：foundation 新增 `safe_truncate`，替换 kernel/logging.rs、task/decompose.rs、gateway/telegram_format.rs 三处字节切片；修 logging 密钥前缀泄漏。补中文/emoji 回归测试。
2. P0-2：path_policy canonicalize 失败改拒绝；补 `..` 穿越测试。
3. P0-3：evolution/proposal_lifecycle/proposal_processing/curation 全部改 fail-closed（default→Missing/false；readiness 必须 Some(true)）；live_orchestrator 补真实 lease 校验。
4. P0-5：autonomy 脱敏改结构白名单并统一到契约边界（内存/JSONL 同路径）。
5. P0-1（第一步止血）：ShellTool/FileRead/FileWrite/skill execute_shell_entry 先加路径白名单 + 输出上限 + 超时 kill + 完整命令行脱敏；trace 必填。
6. B1/B2/B3：heartbeat coalesce 护栏、scheduler `scheduled_for<=now` 过滤、run-id 零填充（含存量迁移或版本化）。

### 阶段 2：门禁与观测面（2–3 天）
1. R3 共享脱敏模块落地，替换 llm 4 个 provider、memory embedding/vector、gateway 日志、tools trace 中间件（S7–S11）。
2. G2：sdk_no_provider_construction_gate 改正则+登记元测试；加「禁止 concat! 拆字面量绕过」检查；G3 增 use 级扫描。
3. 假成功家族改结构化 unavailable/unsupported：gateway telegram/discord、tools ListAgents、driver loader、skill facade、scheduled-agent-task 回滚（S12–S18）。
4. K1：event_log append/index 写错误传播，修复重放空洞。

### 阶段 3：副作用门禁与 trace 闭环（3–5 天，OpenSpec 提案先行）
1. R1 SideEffectGuard 装饰器落地；task 写命令、skill/tools 执行、gateway 出入站全部接入（S19/S22/P0-1 终态）。
2. S21 专项确认：LLM/driver/context 的 runtime-host 装饰强制点；封死 `impl LlmProvider for LlmRouter` 直调绕过。
3. S24 scheduler scope 校验；S25 context policy 传递；S23 skill 命令 trace 必填。
4. R6 LLM 工厂化 + 预算注入（S1–S3、S20）。

### 阶段 4：状态机与并发正确性（3–5 天）
1. R4 迁移矩阵接入 task/scheduler/heartbeat（B6），heartbeat/scheduler 各补状态机单测。
2. B4 幂等 TOCTOU、B5 claim CAS、B7 goal 崩溃恢复、B9 worker 超时、B10 queue 结果回写、B22 graph 准入。
3. B11/B12 cron 语义与 stagger 漂移；B16 telegram 忙循环退避。
4. B13/B14 driver FFI runtime 与回调生命周期；B15 上下文截断成对保留。
5. B8 runtime-host 六锁死锁风险。

### 阶段 5：边界重构（1–2 周，逐项 OpenSpec）
1. R2 domain_pack_contract 拆 crate + 领域注册表数据驱动 + SDK 表驱动测试瘦身（A1/A2/A5/K8）。
2. R5 Shell 语义收编（W1–W6）+ CLI 下沉 SDK 移除 reqwest（W4/W9）。
3. K2–K7：proto 默认配置中性化、web3/nats feature 门控、alert 便捷方法上移、payment_store 迁出、execution_port 锁修复。
4. A3 DashScopeFormatter 迁出 framework；A4 macaca-app 依赖收敛；S4/S5/S6/S26/S27 硬编码与 autonomy 语义下沉。
5. W7 service_router 幂等感知重试（payment/evm 高危，建议提前到阶段 3 一并做）。

### 阶段 6：横向卫生与生命周期（持续）
1. R7：锁毒化 191 处 expect 收敛、无界增长家族加保留策略、unbounded channel 评估（B17–B19）。
2. 服务契约补全：tools ServiceContract、gateway 生命周期/CancellationToken、autonomy/skill 生命周期算子、health 真实探测（4.3 全部）。
3. 近限文件拆分（第五章清单，evm_service_provider 优先）；死代码清理（B20/B21、skill discovery MAX_DEPTH）。
4. 每项完成后更新三部宪法文档的相应条目与本报告状态。

### 验收标准（对齐宪法 Acceptance Gates）
- 全部门禁测试绿灯（含新增的正则构造门禁、use 级扫描、fail-closed 单测）。
- 中文/emoji 输入下 logging/decompose/telegram 无 panic（新增回归测试）。
- provider 缺失场景全链路返回结构化 unavailable（gateway/tools/driver 各补一条集成测试）。
- 非幂等服务命令在注入瞬时失败下不重复副作用（service_router 幂等测试）。
- grep 门禁确认 OS 层无新增 provider/model/应用名分支。

---

## 八、需人工确认事项

1. **LLM/driver/context 的 trace/policy 强制点**：三者为 provider 实现层，装饰按宪法归属 runtime-host；但存在可直调绕过路径（S21），需确认所有入口是否强制经装饰器。
2. **Milvus 后端无鉴权 header**（memory/vector.rs）：确认是否仅面向无鉴权本地实例；若将对接远程需补认证与 TLS。
3. **macaca-app 的 LlmProxy 中介地位**（A4）：是保留并 OpenSpec 记录边界，还是改经 SDK/服务边界。
4. **macaca-task/src/tracker.rs** 是否确为遗留死代码（B21）。

---

## 附录：各层合规确认（本次审计未发现问题的项）

- 内核依赖方向、trace-required 中间件、无静态状态违规、生产路径无 unwrap。
- SDK 依赖纯度与 provider 隔离；finance domain pack 外部 URL 归属正确。
- web/cli 依赖收敛（CLI reqwest 除外）；service_router policy 先行与 hash 脱敏；provider 构造均在组合根。
- 六类宪法门禁均有可执行测试；生产 panic!/todo!/unimplemented! 为 0；生产代码无锁跨 await 违规（std Mutex 场景）；生产 unwrap 61 处均已核实为安全场景（metrics 启动期、FFI catch_unwind 内）。
- context/skill/memory 多处正确范式可作全仓参照：context 的"引用+计数+短码"报告设计、memory 的 redact_text+resilience、skill projection 的 symlink 防护、llm resolver 的 descriptor 数据表。
