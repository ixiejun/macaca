# Macaca Agent 自进化 Skill 系统研究

> 目标：基于 `/Users/quantum/Code/dev/agent/hermes-agent` 源码、Hermes Curator 官方文档与 Jdon 文章，研究 Hermes 的 agent 自进化与 skill 策展机制，并在 Macaca 三个开发宪法约束下探索 Macaca Agent OS 的自进化 skill 能力。

## 1. 核心结论

Macaca 不能只把 skill 看成 prompt 片段或工具目录。对于 7x24 小时自治运行的 Agent OS，skill 应成为一种 **可治理的程序性经验资产**：

- agent 在完成任务后，可以把可复用经验沉淀为 skill。
- skill 后续可以被使用、修订、拆分、合并、降级为 reference/template/script。
- skill 生命周期必须可审计、可恢复、可追踪，不能让 agent 写完就永久污染能力目录。
- skill 的增长必须由系统服务治理，避免 skill 爆炸、重复技能、过细 session artifact 和 stale knowledge。

推荐的 Macaca 目标形态是：在现有 `macaca-skill`、Skill service、context composer、memory/context/task 服务基础上，引入 **Skill Evolution Service** 与 **Skill Curation Service**。它们不是 kernel 功能，也不是 Web/CLI 语义，而是 Skill / Memory / Context / Task / Policy / Store 协作的系统服务能力。

一句话：Macaca 的 agent 应该越来越聪明，但“变聪明”的过程必须像操作系统管理软件包、日志、快照和策略一样被治理。

## 2. 参考资料与证据范围

### 2.1 Hermes 源码路径

本次重点阅读：

- `hermes-agent/agent/curator.py`
- `hermes-agent/agent/curator_backup.py`
- `hermes-agent/tools/skill_manager_tool.py`
- `hermes-agent/tools/skill_usage.py`
- `hermes-agent/tools/skills_tool.py`
- `hermes-agent/hermes_cli/curator.py`
- `hermes-agent/tests/agent/test_curator*.py`
- `hermes-agent/tests/hermes_cli/test_curator*.py`

### 2.2 Hermes 文档与文章

- Jdon 文章：<https://www.jdon.com/91711-hermes-curator-saves-merge-skills.html>
- Hermes Curator 官方文档：<https://hermes-agent.nousresearch.com/docs/user-guide/features/curator>
- Hermes Skills System 官方文档：<https://hermes-agent.nousresearch.com/docs/user-guide/features/skills>

### 2.3 Macaca 约束文档

本研究以以下三个宪法为硬边界：

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`

同时参考：

- `macaca/docs/design_patterns.md`
- `openspec/changes/add-agent-skills-runtime/design.md`
- `openspec/changes/add-skills-mcp-capability-context/design.md`
- `openspec/changes/migrate-skill-consumers-to-pattern-primitives/design.md`

## 3. Hermes 自进化系统分析

### 3.1 Hermes skill 的基础模型

Hermes 使用 AgentSkills-compatible 目录结构：

```text
~/.hermes/skills/<skill-name>/
├── SKILL.md
├── references/
├── templates/
├── scripts/
└── assets/
```

`tools/skills_tool.py` 体现了 progressive disclosure：

- `skills_list` 只返回 name、description、metadata。
- `skill_view` 在需要时读取 `SKILL.md` 或支持文件。
- `SKILL.md` 使用 YAML frontmatter，要求 `name` 与 `description`。
- 支持 `references/`、`templates/`、`scripts/`、`assets/` 作为二级知识载体。

这和 Macaca 已有 `AgentSkill` 方向一致：skill 正文不是默认全量注入，而是由模型根据 catalog 选择后按需读取。

### 3.2 Hermes 的 skill 生成与更新

`tools/skill_manager_tool.py` 让 agent 可以管理 skill：

- `create`：创建新的 `SKILL.md`。
- `edit`：整体替换 `SKILL.md`。
- `patch`：对 `SKILL.md` 或支持文件做局部替换。
- `write_file`：写入 `references/`、`templates/`、`scripts/`、`assets/`。
- `remove_file`：删除支持文件。
- `delete`：删除 skill，并支持 `absorbed_into` 标记内容被哪个 umbrella skill 吸收。

关键经验：

- skill 是程序性记忆，不等同于普通 memory。
- 支持文件是一等结构，避免所有经验都挤进 `SKILL.md`。
- 写入使用原子替换和格式校验。
- agent-created skill 可以经过安全扫描，但 Hermes 默认对 agent-created scan 关闭，因为 agent 本来就能通过 terminal 执行相同风险路径。

Macaca 不应照搬这个安全取舍。Macaca 是 OS，应把 skill 写入、更新、执行依赖、支持文件写入都纳入 policy、trace、audit、entitlement 与 package guard。

### 3.3 Hermes 的 usage/provenance 设计

`tools/skill_usage.py` 维护 `~/.hermes/skills/.usage.json` sidecar。它记录：

- `created_by`
- `use_count`
- `view_count`
- `patch_count`
- `last_used_at`
- `last_viewed_at`
- `last_patched_at`
- `created_at`
- `state`
- `pinned`
- `archived_at`

重要点：

- usage telemetry 不写进 `SKILL.md` frontmatter，避免污染作者内容。
- 使用文件锁和原子写，避免并发写坏 sidecar。
- bundled / hub-installed skill 不参与 agent-created curation。
- agent-created 资格不是简单由目录推断，而是 usage record 中显式标记 `created_by = agent`。

这对 Macaca 很关键。Macaca 需要把 “skill 内容” 与 “skill 运行时治理元数据” 分开：前者属于 skill package，后者属于 Skill Governance Store。

### 3.4 Hermes Curator 的运行机制

`agent/curator.py` 定义 Curator：后台 skill 维护 orchestrator。

核心机制：

- 默认一周运行一次，且需要 agent idle 一段时间。
- 首次观察不立即运行，而是 seed `last_run_at`，延迟一个完整 interval，给用户 dry-run、pin、disable 的机会。
- 运行分两段：
  - deterministic auto transitions：无 LLM，按 `stale_after_days`、`archive_after_days` 转换 active/stale/archived。
  - LLM review pass：forked auxiliary agent 扫描 agent-created skills，执行 patch/create/write_file/delete。
- 只处理 agent-created skills。
- pinned skills 跳过自动转移，并拒绝 delete。
- 永不真正自动删除，最重操作是 archive 到 `.archive/`。
- 每次真实运行前创建 snapshot，可 rollback。
- 每次运行生成 `run.json` 与 `REPORT.md`。
- consolidation 后改写 cron jobs 中的 skill 引用，避免计划任务下次加载失效 skill。

### 3.5 Hermes Curator 的“umbrella skill”思想

Curator prompt 中最有价值的判断是：skill 库的目标不是“每个 session 的 bug 都变成一个独立 skill”，而是形成 **class-level skills**：

- 一个宽泛 umbrella skill 覆盖一类工作流。
- session-specific 细节进入 `references/`。
- starter artifacts 进入 `templates/`。
- 可重复执行动作进入 `scripts/`。
- 过细、过窄、PR 编号、具体错误字符串、一次性 audit 产物，都应合并或降级。

这是防止 skill 爆炸的关键。

Macaca 应吸收这个思想，但需要更 OS 化：Curator 不应只靠 prompt 规则，而应有可执行的 classification、policy、trace 和 rollback contract。

## 4. Hermes 方案的局限

Hermes Curator 已经解决了“技能越学越乱”的核心问题，但从 Macaca OS 视角看仍有几个局限：

1. **文件系统是主要真相源**
   - Hermes 使用 `~/.hermes/skills/`、`.usage.json`、`.curator_state`、`.archive/`。
   - Macaca 应保留人类可读文件，但治理真相应进入 service store / event log / audit chain。

2. **curation 主要由一个 prompt 驱动**
   - Hermes 的 prompt 很强，但结构化决策仍依赖模型输出与工具调用后验分类。
   - Macaca 应把候选聚类、相似度、usage stats、policy check、risk score、approval gate 建成 typed commands。

3. **provenance 模型偏简单**
   - 官方文档说 agent-created 包括 agent 保存、手写、external dirs；源码中已经更倾向显式标记 agent-created。
   - Macaca 需要更细：author_kind、scope、package source、trust level、application/agent/session provenance、evidence refs。

4. **安全边界还不是 OS 级**
   - Hermes 依赖 CLI/config 与工具层校验。
   - Macaca 必须走 policy、capability declaration、resource/entitlement、trace-required service calls。

5. **cron rewrite 是后处理**
   - Hermes consolidation 后改写 cron skill refs。
   - Macaca 应把 skill identity 与 alias/redirect/absorbed_into 建模成 service-level mapping，所有 Task/Scheduler/Context consumers 通过 Skill service 解析。

## 5. Macaca 当前基础

Macaca 已经具备可承接这套能力的基础：

- `macaca-skill/src/agent_skill.rs` 已支持 AgentSkills-compatible `SKILL.md`、source scope、metadata、progressive disclosure。
- `macaca-runtime-host/src/skill_service_provider.rs` 已有 Skill system service provider，提供 snapshot、executable load、tool catalog、tool invoke、status、snapshot、cleanup。
- `macaca-sdk/src/skill_client.rs` 已有 `SystemSkillClient` facade 与 unavailable Null Object 行为。
- `macaca-context/src/capability/skill_provider.rs` 已把 frozen skill catalog 作为 context provider 输入，且不直接读磁盘。
- `add-agent-skills-runtime` 设计已明确 skill snapshot、session/run freeze、trace 事件和 prompt catalog。
- `add-skills-mcp-capability-context` 设计已把 skill/MCP 能力统一成 compact capability candidate，而不是全量注入。

因此，自进化 skill 不应从零开始，也不应绕过这些服务。正确路径是扩展现有 Skill service 的治理面，并让 Task/Memory/Context/Policy/Store 服务参与。

## 6. 设计原则：Macaca 版本不能变成 Hermes 文件脚本

### 6.1 宪法映射

| Macaca 宪法要求 | 对自进化 skill 的约束 |
| --- | --- |
| kernel 只拥有 invariant | kernel 只知道 skill identity、capability name、service call、trace/audit，不生成、不合并、不读写 skill 内容 |
| service owns replaceable capability | Skill Evolution / Curation 必须是系统服务，可替换 provider |
| application owns product behavior | application 可以声明 agent skill scope 和 policy，但不能让 OS 写死业务 skill |
| shell is adapter | Web/CLI 只能触发 run、展示报告、审批，不拥有 curation 逻辑 |
| no hardcoded workflow/app/provider names | skill 聚类、合并、策略都基于 metadata/capability，不按 app name 分支 |
| trace/policy required | 每次 skill create/patch/archive/consolidate 必须有 trace、policy、audit |
| optional modules may be absent | vector similarity、LLM curator、marketplace、remote store 缺失时返回 unavailable/unsupported，而不是 fake success |

### 6.2 必须使用的设计模式

- **Facade**：SDK 暴露 `SystemSkillEvolutionClient` / `SystemSkillCurationClient`。
- **Command**：所有写入与策展动作都是 typed command/result。
- **Strategy**：curation provider、similarity provider、approval policy、archive policy 可替换。
- **Decorator**：trace、policy、resource、entitlement、budget 包裹 service call。
- **State**：skill lifecycle 显式建模。
- **Observer**：usage/view/activation/patch/consolidation 事件进入 event bus。
- **Memento**：pre-run snapshot、rollback、run report、alias map 可回放。
- **Specification**：admission、size limit、trust、scope、merge eligibility 是可执行规则。
- **Abstract Factory**：provider/module bootstrapping 在 runtime-host composition root。

## 7. 推荐目标架构

### 7.1 能力分层

```text
Application / Agent
  declares skill scope, policy, lifecycle preference

Context Composer
  injects compact skill/capability index and observes skill reads

Task / Autonomy Services
  request experience extraction after verified task completion

Skill Evolution Service
  turns proven task experience into draft skill patches or new skills

Skill Curation Service
  merges, archives, aliases, snapshots, reports, and rolls back skills

Skill Runtime Service
  discovers, snapshots, activates, loads resources, exposes executable tools

Store / Memory / Knowledge Services
  persist governance metadata, provenance, evidence, semantic index

Kernel
  identity, service registry, policy facade, trace/audit bus only
```

### 7.2 建议新增服务

#### Skill Evolution Service

职责：

- 从完成任务中提取可复用经验。
- 判断该经验应该进入 memory、knowledge digest、existing skill patch、new draft skill，还是无需沉淀。
- 生成 draft，不默认立即生效。
- 把 proof、task trace、artifact refs、失败修复路径和验证命令关联到 draft。

典型命令：

```text
skill.evolution.propose_from_task
skill.evolution.propose_patch
skill.evolution.promote_draft
skill.evolution.reject_draft
skill.evolution.snapshot
```

#### Skill Curation Service

职责：

- 维护 skill lifecycle。
- 统计 usage/view/read/patch/activation。
- 发现重复、过窄、过期、冲突和过大 skill。
- 执行 umbrella merge、support-file demotion、archive、alias/redirect。
- 生成 report 与 rollback memento。
- 修正 Task/Scheduler/Context 中的 skill references，或通过 alias map 透明解析。

典型命令：

```text
skill.curation.status
skill.curation.run
skill.curation.dry_run
skill.curation.pin
skill.curation.unpin
skill.curation.archive
skill.curation.restore
skill.curation.rollback
skill.curation.resolve_alias
skill.curation.snapshot
```

### 7.3 不建议新增 kernel 能力

kernel 不应出现：

- `Curator`
- `SkillMerger`
- `SkillUsageStore`
- `SkillAuthor`
- `SkillEmbeddingProvider`
- `SkillMarketplaceClient`

kernel 只需要能路由 typed service call，记录 trace/audit，并让 policy facade 做允许/拒绝判断。

## 8. 数据模型建议

### 8.1 SkillLifecycleState

```text
Draft
Active
Stale
Archived
Quarantined
Superseded
Rejected
```

说明：

- `Draft`：agent 提议但未激活。
- `Active`：可被 context composer 选入 catalog。
- `Stale`：低活跃或长期未验证。
- `Archived`：不进入 catalog，但可恢复。
- `Quarantined`：风险、注入、越权、依赖异常。
- `Superseded`：被 umbrella skill 吸收，保留 alias/redirect。
- `Rejected`：明确不采纳的经验提案。

### 8.2 SkillProvenance

```text
skill_id
version
author_kind: agent | user | bundled | marketplace | application | plugin
author_agent_id
application_id
session_id
task_id
trace_id
evidence_refs
created_at
updated_at
source_scope
trust_level
```

### 8.3 SkillUsageTelemetry

```text
skill_id
scope
view_count
activation_count
resource_read_count
patch_count
successful_task_count
failed_task_count
last_viewed_at
last_activated_at
last_patched_at
last_success_at
last_failure_at
```

Macaca 应比 Hermes 多一个 `successful_task_count` / `failed_task_count`。使用次数不等于有效性；一个被频繁错误触发的 skill 应降低权重或进入修订。

### 8.4 SkillCurationRun

```text
run_id
trace_id
started_at
finished_at
provider_id
dry_run
candidate_count
actions
before_snapshot_ref
after_snapshot_ref
report_ref
rollback_ref
policy_decisions
audit_event_ids
```

### 8.5 SkillAliasMap

```text
from_skill_id
to_skill_id
reason
created_by_run_id
valid_from
valid_until
resolution_policy: redirect | warn | deny
```

这样 Scheduler、Task、Context 不需要被动改写所有历史引用；服务解析时可统一处理。

## 9. 自进化工作流

### 9.1 任务完成后的经验提取

触发条件不应是“模型觉得值得保存”，而应是已验证任务闭环：

1. Task service 标记任务 terminal success。
2. Evidence gate 验证 artifact、test、trace 或外部状态。
3. Skill Evolution Service 读取 bounded task summary、trace digest、memory digest。
4. 生成 `ExperienceCandidate`。
5. Classification：
   - declarative fact -> Memory/Knowledge。
   - reusable procedure -> Skill draft。
   - project-only convention -> project skill or project memory。
   - one-off detail -> task report，不沉淀为 skill。
6. 策略决定：
   - patch existing umbrella skill。
   - create new draft。
   - write support file。
   - discard。
7. 进入 policy/approval。
8. 激活后进入 skill catalog 的下一次 snapshot。

### 9.2 周期性 curation

参考 Hermes，但 OS 化：

1. Scheduler/Autonomy service 发起 curation wake。
2. Curation service 检查 interval、idle、budget、policy。
3. 创建 memento snapshot。
4. deterministic phase：
   - stale/archive/quarantine candidates。
   - size limit / invalid metadata / missing dependency diagnostics。
5. analysis phase：
   - usage stats。
   - semantic similarity。
   - prefix/domain cluster。
   - failure/success correlation。
   - reference graph。
6. review phase：
   - LLM curator provider 给出 typed proposal。
   - 不直接执行危险动作。
7. apply phase：
   - policy approved actions 才写入。
   - pin/quarantine/bundled/marketplace trust 规则先行。
8. report phase：
   - run.json/report.md/event log。
   - alias map。
   - rollback ref。

### 9.3 什么时候需要人工审批

默认可自动执行：

- Draft 经验进入 private agent skill draft。
- stale 标记。
- dry-run report。
- support-file 写入 draft。

需要 policy 或用户审批：

- active skill patch。
- active skill archive。
- umbrella merge。
- marketplace/bundled/application-owned skill 修改。
- 会影响多个 application/agent 的 central skill 修改。
- 任何 executable script 写入或修改。
- 删除、不可逆迁移、跨 tenant skill 共享。

## 10. 防止 skill 爆炸的规则

### 10.1 skill 不应保存的内容

不应沉淀为 skill：

- 单个任务的完整日志。
- 某次失败的全部原始 stack trace。
- PR 编号、临时分支名、一次性 artifact 路径。
- 只对一个 app 的一次性业务判断。
- 用户隐私、凭证、raw prompt、provider raw payload。
- 没有验证过的猜测。

这些内容应进入 task report、memory evidence、knowledge digest 或 trace，而不是 skill。

### 10.2 新 skill 的准入门槛

创建新 active skill 前必须满足：

- 可复用任务类别明确。
- description 能触发未来任务匹配。
- 不能被现有 umbrella skill 的一个 subsection 覆盖。
- 有 evidence refs 表明该流程至少成功一次。
- 没有包含敏感信息。
- 有 source scope 和 ownership。

### 10.3 Umbrella merge 规则

应该合并：

- 多个 skill 共享同一领域、同一操作流或同一失败修复模式。
- skill 名称包含一次性 artifact、PR、error string、audit、diagnosis、salvage。
- skill 正文大部分是 session-specific 细节，可降级为 `references/`。
- 使用者需要的是“某类任务怎么做”，不是“某次任务发生了什么”。

不应该合并：

- capability / permissions 明显不同。
- 安全等级不同。
- scope 不同，例如 agent private skill 与 application shared skill。
- 一个是 knowledge skill，另一个是 executable skill。
- 一个由 marketplace/bundled 管理，另一个由 agent 私有管理。

## 11. 与 Memory / Knowledge 的边界

Skill、Memory、Knowledge 需要分工：

| 类型 | 内容 | 例子 |
| --- | --- | --- |
| Skill | 可执行/可复用流程 | “调试 heartbeat live proof 时要先验证 artifact mtime 和 content” |
| Memory | 事实、偏好、上下文 | “用户要求 tasks.md 必须保持真实” |
| Knowledge | 带证据的结构化知识 | “Macaca heartbeat proof 需要 Scheduler wake、agent execution、durable artifact 三类证据” |
| Trace | 发生过的事件 | 某次 service call、tool call、test output |
| Task Report | 一次任务交付记录 | 本次实现了哪些文件、验证了哪些命令 |

经验提取服务必须能把同一任务中的不同信息分流到不同系统，而不是统统写成 skill。

## 12. 与 Context Composer 的关系

自进化 skill 最终影响 agent 智能水平的路径是 context composer：

- Curation 后的 active skill catalog 进入 compact capability index。
- Skill 正文仍按需读取。
- Superseded skill 通过 alias map 解析到 umbrella。
- Stale/archived/quarantined skill 不进入普通 catalog。
- Draft skill 默认不进入生产 agent catalog，除非在实验 profile 中启用。
- Context report 应显示：
  - 本轮可见 skill 数。
  - 被过滤 skill 数及原因。
  - 已解析 alias。
  - skill read/activation trace。

这保持了 Macaca 当前 `SkillContextProvider` 的正确方向：context 层消费 frozen snapshot，不直接读磁盘或拥有 discovery。

## 13. 与 Package / Store / Entitlement 的关系

长期看，skill 是 package type 之一。自进化系统必须尊重 package ownership：

- bundled skill：由 Macaca 发布节奏管理，不允许 agent 自动 patch。
- marketplace skill：由 upstream 管理，agent 可提本地 overlay/draft，但不能伪装成 upstream 更新。
- application skill：由 application package 管理，修改需要 app scope policy。
- agent private skill：agent 可在 policy 内自动演化。
- central user skill：需要用户或 tenant-level policy。

如果未来支持 paid skill 或 encrypted skill，curation 只能操作 metadata/usage/alias，不能绕过 entitlement 解密或修改。

## 14. OpenSpec 候选拆分

建议后续不要一个大提案吞掉全部能力。可以拆成四个变更：

### Change 1: add-skill-governance-store

目标：

- 引入 SkillProvenance、SkillUsageTelemetry、SkillLifecycleState、SkillAliasMap。
- Skill service 暴露治理 snapshot/status。
- Context composer 使用 alias/lifecycle 过滤 catalog。

### Change 2: add-skill-experience-evolution-service

目标：

- 从 verified task/evidence 中生成 skill draft 或 patch proposal。
- 与 Memory/Knowledge 分流。
- draft 默认不激活。

### Change 3: add-skill-curation-service

目标：

- dry-run、status、pin/unpin、archive/restore、merge proposal、report、rollback。
- deterministic stale/archive。
- provider-backed LLM review，但 typed proposal 与 policy apply 分离。

### Change 4: add-skill-curation-operations-ui

目标：

- Web/CLI 只做 shell：展示 report、审批、rollback、pin。
- 不拥有 skill 合并语义。

## 15. 验证与边界测试建议

必须新增 gates：

- kernel 不依赖 skill curation/evolution provider。
- SDK 不构造 curation provider。
- Web/CLI 不写 skill 语义，只调用 facade。
- bundled/marketplace skill 不被 agent 自动 patch/archive。
- pinned skill 不被 archive/delete。
- archived skill 可 restore。
- superseded skill alias 可解析。
- curation dry-run 不写 active store。
- rollback 能恢复 skill tree、usage telemetry、alias map、scheduler refs。
- logs/reports 不含 raw prompts、secrets、provider raw payload。
- absent LLM curator provider 返回 structured unavailable，deterministic phase 仍可运行。

## 16. 推荐落地顺序

1. 先补 governance store，不碰自动生成。
2. 再把 usage/read/activation/patch telemetry 接入 trace。
3. 加 deterministic curation：pin、stale、archive、restore、dry-run report。
4. 加 alias map，避免历史 task/scheduler refs 失效。
5. 加 experience draft：任务完成后只生成 draft，不自动生效。
6. 加 LLM curator provider：只输出 typed proposal。
7. 最后加自动 apply policy 与 operations UI。

这个顺序能保证每一步都是可审计、可回滚、可验证的 OS 能力，而不是一个 prompt 驱动的黑箱。

## 17. 最终判断

Hermes Curator 的价值不是“帮忙删 skill”，而是补上了 agent 自我改进系统的第二半：agent 会积累经验，也必须能整理经验。没有 Curator，自我改进会从能力增长变成能力污染。

Macaca 应吸收 Hermes 的三个核心思想：

- self-improvement loop 产出的 skill 需要生命周期治理。
- 过细 skill 应合并进 class-level umbrella skill。
- archive、snapshot、report、rollback 是自动整理系统的安全底线。

但 Macaca 不能照搬 Hermes 的文件脚本模型。Macaca 的正确形态是 OS 级服务：

- Skill Evolution Service 负责经验沉淀。
- Skill Curation Service 负责整理优化。
- Skill Runtime Service 负责发现、snapshot、activation。
- Context Composer 负责把整理后的能力以 compact index 进入模型上下文。
- Memory/Knowledge 服务负责保存事实和证据。
- Policy/Trace/Audit/Store/Entitlement 负责让整个过程可控、可查、可恢复。

这样 Macaca application 里的 agent 才会在长期任务中真正越来越聪明，而不是只是越跑越臃肿。
