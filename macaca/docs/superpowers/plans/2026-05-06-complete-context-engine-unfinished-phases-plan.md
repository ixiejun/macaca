# Superpowers Brainstorm + Write-Plan：完善 Context Engineering 未完成 Phases

日期：2026-05-06

## 输入与当前状态

本计划基于以下事实来源：

- 研究报告：`docs/context-engineering-openclaw-hermes-research.md`
- 当前状态矩阵：`openspec/changes/complete-context-engine-runtime-phases/phase-status.md`
- 最终收口提案：`openspec/changes/complete-context-engine-all-phases/`
- 设计模式参考：`macaca/docs/design_patterns.md`

最近一次审计结论：

- 研究报告原始 `Phase 0-5` 中，`Phase 0`、`Phase 1`、`Phase 5` 已基本闭环。
- `Phase 2`、`Phase 3`、`Phase 4` 已有基础能力，但还缺产品级闭环。
- 后续 OpenSpec 已把这些缺口扩展为 `Phase 6-10` 收尾项。
- 当前最该推进的载体是 `complete-context-engine-all-phases`，而不是继续扩写多个旧 change。

## 约束

- Macaca 是 Agent OS 基础设施，不能写 app/workflow/driver/provider 特化逻辑。
- 所有上下文、记忆、技能、MCP、插件能力必须通过可替换接口或 adapter 接入。
- 不删除 legacy API；只标记 deprecated、迁移生产调用、保留可搜索性。
- 不引入新的 crate；优先在 `macaca-context`、`macaca-web`、`macaca-persist` 内做模块化文件拆分。
- 文件保持小而清晰，超过 500 行必须拆分。
- 行为变更先更新 OpenSpec，再小步实现和验证。

## 设计模式选择

- **Facade**：上层继续只依赖 `ContextFacade` / runtime-facing facade，不直接拼 prompt。
- **Adapter**：pruned source retrieval、lineage store、wiki/digest recall、external provider 都通过 adapter 接入。
- **Strategy**：pruning render policy、lineage display policy、recall policy、external fallback policy 可替换。
- **Decorator**：redaction、tombstone、trust fencing、timeout、circuit breaker 包裹外部输入。
- **Repository**：canonical EventLog/session/artifact 原文读取走 repository 抽象，不让 UI 或 context engine 直接读底层 key。
- **Chain of Responsibility**：memory/wiki recall、profile、skills、MCP、tool schema 仍作为 provider chain 进入 composer。
- **Memento**：compaction successor、lineage root/tip、summary segment 是可审计状态快照。
- **Bridge / Ports and Adapters**：用户自定义 context engine/provider 与运行时解耦，外部 adapter 只实现端口。

## Superpowers Brainstorm

### 方案 A：只完成研究报告 Phase 2/3/4 的缺口

范围：

- Phase 2：补 pruned source 原文可取回。
- Phase 3：补 lineage UI。
- Phase 4：补 wiki/digest recall request-only 闭环。

优点：

- 交付面小，直接解决用户指出的未完成 phase。
- 风险较低，不碰插件外部 adapter 和归档纪律。

缺点：

- 会留下 `complete-context-engine-all-phases` 中 Phase 9/10 继续 partial。
- 后续还是需要二次 OpenSpec/计划。

结论：

- 适合作为第一批实施 slice，但不是最终完成方案。

### 方案 B：按 `complete-context-engine-all-phases` 一次性规划 Phase 6-10

范围：

- Phase 6：Non-destructive pruning retrievability closure。
- Phase 7：Compaction lineage UX closure。
- Phase 8：Memory/wiki digest recall runtime closure。
- Phase 9：Custom engine/provider + external adapter path。
- Phase 10：Migration/deprecation/archive discipline。

优点：

- 与现有 OpenSpec 完全对齐。
- 能把“研究报告未完成项”和“后续扩展 phase”一次性纳入统一门禁。
- 避免多个 partial change 长期悬挂。

缺点：

- 范围大，需要严格切片，否则容易变成 mega refactor。
- Phase 9 external adapter 容易过度设计。

结论：

- 推荐采用，但必须拆成小步实施。

### 方案 C：先实现外部插件体系，再反向补 Phase 2/3/4

范围：

- 先做 custom/external context provider 协议。
- 再让 pruning/lineage/wiki 作为插件样例。

优点：

- 插拔性最强。

缺点：

- 本末倒置；当前缺口是已有内建能力未闭环。
- 外部协议会在内部 contract 尚未稳定前被过早冻结。
- 风险高、验证难。

结论：

- 不推荐。

## 推荐方案

采用 **方案 B**，但执行上按以下原则降风险：

- 先补已有能力闭环：Phase 6、Phase 7、Phase 8。
- 再做插拔扩展：Phase 9。
- 最后做迁移与归档：Phase 10。
- 每个 Phase 都必须满足四类完成证据：Contract、Runtime、Diagnostics、Verification。

## Write-Plan

### Step 0：OpenSpec 与状态矩阵对齐

目标：

- 让 `complete-context-engine-all-phases` 成为唯一收口入口。
- 把 Phase 0-5 和 Phase 6-10 的状态写清楚，避免“partial 被误判 complete”。

任务：

- 更新 `complete-context-engine-all-phases/tasks.md` 的 baseline audit 条目。
- 更新或替换 `complete-context-engine-runtime-phases/phase-status.md`，明确：
  - Phase 0/1/5 已完成。
  - Phase 2/3/4 的剩余缺口分别由 Phase 6/7/8 收口。
  - Phase 9/10 是后续扩展与归档门禁。
- 运行 `openspec validate complete-context-engine-all-phases --strict`。

验收：

- 状态矩阵与本计划一致。
- 未完成项不再散落在多个旧 change 中无法追踪。

### Step 1：Phase 6 Non-Destructive Pruning Retrievability

目标：

- 被裁剪进入模型上下文的内容必须保留 canonical 原文。
- `ContextReport.source_ref/artifact_ref` 必须能指向可授权取回路径，或明确不可取回原因。

设计：

- Repository pattern：新增或收敛 `ContextSourceArtifactRepository`，统一从 EventLog/session/artifact store 读取原文。
- Adapter pattern：不同来源类型分别实现 ref 解析，不把 key 规则硬编码到 UI。
- Strategy pattern：每个 source kind 有 retrieval policy，决定可取回、受限、不可用。

任务：

- 盘点 source kinds：tool result、trace event、command stdout/stderr、file read、search result、skill/capability large payload。
- 确认每类 source 的 canonical 原文位置。
- 为 `ContextReport` source row 补齐稳定 ref 或 unavailable reason。
- 后端增加 debug/API retrieval path，拒绝跨 session ref。
- 前端 context diagnostics 增加“查看原文/不可取回原因”入口。
- 测试覆盖：pruning 不改 canonical payload；每个 supported source kind 原文可取回。

风险：

- 旧 trace/session 存储并非所有 source 都有稳定 key。
- UI 直接展示原文可能泄漏敏感内容。

缓解：

- 取回 API 默认只返回 bounded preview，完整原文仅 debug/authorized 模式。
- ref 解析强制校验 app/session/agent scope。

### Step 2：Phase 7 Compaction And Session Lineage UX

目标：

- 压缩后的 session 在 UI 中仍表现为一个 logical session。
- debug 模式能看到 root、tip、successor chain、summary 和事件。

设计：

- Memento pattern：compaction summary 和 successor segment 作为状态快照保存。
- Facade pattern：UI 调用 lineage API，不理解底层 store。
- Strategy pattern：默认 logical session 读 tip；debug 展开 root-to-tip。

任务：

- 确认 automatic compaction 走 `summary` engine，不只是 manual compact API。
- 确认 `before_compaction` / `after_compaction` hooks 对 memory/source provider 生效。
- 审计所有 session read/list/replay 路径是否默认 resolve lineage tip。
- 前端增加 lineage 视图或 context report 中的等价交互：
  - root session id
  - current tip session id
  - successor chain
  - compaction summary
  - compact events
- 测试覆盖：
  - compacted session 可继续 resume。
  - 原始历史仍可读。
  - summary 是 reference-only/untrusted。
  - 默认 UI 不丢 logical session。

风险：

- 修改 session 读取路径可能影响左侧 session logs。
- 自动 compaction 可能改变用户可见会话顺序。

缓解：

- 默认仍展示 logical session root；只在 debug/lineage panel 展开内部 segment。
- 先补 API/UI，再谨慎调整 read path。

### Step 3：Phase 8 Memory Recall And Wiki/Digest Runtime Flow

目标：

- memory recall、wiki/digest recall、active/preflight/explicit recall 统一进入 bounded、dynamic、untrusted、request-only context path。
- 不把 recall 输出写回 canonical transcript。

设计：

- Chain of Responsibility：memory、wiki/digest、skills、MCP 都是 context providers。
- Decorator：tombstone、redaction、privacy、trust fencing、timeout 包裹 recall。
- Adapter：`WorkspaceKnowledgeDigestCapability` 适配 memory runtime 的 knowledge compiler。

任务：

- 实现 runtime wiki/digest recall entry point：
  - 可作为 `KnowledgeDigestContextProvider` runtime provider。
  - 或作为 read-only `wiki_digest_search` / `knowledge_digest_get` 工具。
  - 推荐先 provider path，再考虑工具 path。
- 统一 recall metadata：
  - provenance
  - confidence
  - privacy tier
  - source id
  - evidence ids
- 确认所有 recall candidates 都是 dynamic/untrusted。
- 给 request-only boundary 加测试：
  - provider 不接收可变 transcript。
  - injection 只存在于 assembled request。
  - session store/EventLog 不重复写入 injected recall body。
- 对齐 preflight recall、active recall、explicit tools 的 diagnostics 字段。
- ContextReport/UI 显示 memory/wiki breakdown 和 warnings。

风险：

- wiki digest 和 raw memory 同时注入造成重复。
- digest 内容可能过长或证据链不足。

缓解：

- 复用现有 digest-vs-raw selection。
- 强制 max rows/max chars/evidence depth。
- fail-open：digest provider 失败不阻塞主模型调用。

### Step 4：Phase 9 User Plugin And External Adapter Path

目标：

- 用户能替换 context engine/provider，但不和 Macaca 内部强耦合。
- 外部 adapter 失败不能拖垮主 loop。

设计：

- Ports and Adapters：Macaca 定义 `ContextEngine` / `ContextProvider` port。
- Abstract Factory：从 config/profile 构建 engine/provider family。
- Bridge：runtime selection 与具体 provider 实现分离。
- Anti-Corruption Layer：外部输出必须通过 schema validation、budget、trust fencing。
- Decorator：timeout、max payload、circuit breaker、fallback。

任务：

- 完成 in-process custom engine/provider conformance tests。
- 增加 runtime boot registration path：
  - system default
  - app override
  - agent/profile override
- 文档化 selection precedence。
- 实现最小 external adapter seam：
  - process/RPC/WASM 先定义统一 boundary，不必一次支持所有 transport。
  - 推荐先做 process/RPC mock adapter，WASM 只保留设计位。
- 对外部 payload 强制：
  - schema validation
  - max bytes/max candidates
  - timeout
  - trust level default untrusted
  - circuit breaker
  - fallback to legacy/windowed
- 测试覆盖 custom engine 选择与 external adapter failure degrade。

风险：

- 外部协议过早冻结。
- 安全边界复杂。

缓解：

- 先以 in-process trait 为稳定端口。
- external adapter 标记 experimental，但运行时安全门禁必须完整。

### Step 5：Phase 10 Migration And Deprecation Discipline

目标：

- 旧 prompt/context 入口保留但不再被生产路径调用。
- OpenSpec、代码、测试、GitNexus 证据对齐后才能归档。

设计：

- Facade：生产代码只走 context facade/composer/runtime facade。
- Adapter：legacy API 只作为 compatibility adapter。
- Template Method：统一迁移审计流程，避免每次手写检查。

任务：

- 盘点 legacy prompt/context 入口：
  - direct system prompt builders
  - direct `LegacyContextEngine` production usage
  - direct transcript mutation helpers
  - deprecated catalog prompt helpers
- 给剩余 compatibility API 添加 `#[deprecated]` 或 rustdoc deprecation 指引。
- 生产调用迁移到 facade；测试和 compatibility adapter 可保留。
- 增加 audit test 或 `rg` 脚本：
  - no non-test production path uses deprecated prompt/context entry points
  - legacy APIs remain searchable
- 更新 tasks/status matrix。
- 运行 GitNexus detect-changes 并记录影响范围。

风险：

- Rust `#[deprecated]` 可能让 `-D warnings` 环境失败。

缓解：

- 对 public API 用 `#[deprecated(note = "...")]`。
- 对会影响现有测试的内部兼容入口，至少 rustdoc 明确 replacement，并用 audit test 限制生产调用。

## 最终验证矩阵

每个 Phase 完成前必须有以下证据：

| Phase | Contract | Runtime | Diagnostics | Verification |
|-------|----------|---------|-------------|--------------|
| Phase 6 | retrieval contract/spec | source refs resolve | API/UI original preview | source-kind tests |
| Phase 7 | lineage spec | auto/manual compaction | lineage panel/API | resume/history/reference-only tests |
| Phase 8 | recall/digest spec | provider/tool runtime path | recall breakdown UI/API | opt-in/bounded/request-only tests |
| Phase 9 | plugin/adapter contract | config-selected custom engine | fallback diagnostics | conformance/failure tests |
| Phase 10 | deprecation spec | production migration | audit report | rg/GitNexus/OpenSpec/tests |

## 推荐实施顺序

1. `complete-context-engine-all-phases` OpenSpec/status matrix 对齐。
2. Phase 6 pruning retrievability。
3. Phase 7 lineage UX。
4. Phase 8 wiki/digest recall runtime。
5. Phase 9 custom engine/provider and external adapter seam。
6. Phase 10 migration/deprecation/archive gate。
7. 全量验证与归档准备。

## 必跑验证

```bash
openspec validate complete-context-engine-all-phases --strict
cd macaca
cargo fmt --check
cargo test -p macaca-context --lib
cargo test -p macaca-web --lib
cargo test -p macaca-runtime --lib
cargo test -p macaca-framework --lib
cargo test -p macaca-memory --lib
cargo test -p macaca-persist --lib
cargo test -p macaca-integration-tests
cargo check -p macaca-context -p macaca-runtime -p macaca-framework -p macaca-web -p macaca-memory -p macaca-persist
cd ../frontend
npm run lint
npm run build
cd ..
npx gitnexus detect-changes --repo agent
```

## 不做事项

- 不把 memory/wiki/skills/MCP 逻辑直接写进 prompt string builder。
- 不做 app-specific context policy。
- 不默认注入所有 memory、wiki、skill body 或 MCP resource body。
- 不把 external adapter 做成必需依赖。
- 不删除 legacy API。

