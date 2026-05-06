# Superpowers Brainstorm + Write-Plan：补全 `2026-05-05-context-memory-skills-mcp-integration-plan` 全部 Phases

日期：2026-05-06  
基线：`docs/superpowers/plans/2026-05-05-context-memory-skills-mcp-integration-plan.md`  
现状：composer / profile / active recall / skills+MCP capability / digest / governance / web facade 主轴已 landing；审计结论见最近一次对话（Phase 0 未单列 change id、Phase 2 frontmatter、Phase 3 召回 tombstone、`CompiledContext` 命名、跨切面 stable-hash 证据不足等）。

---

## 1. Superpowers Brainstorm

### 1.1 硬约束（自计划 + Macaca AGENTS）

- OS 底座层禁止应用名/workflow/driver/业务 provider 硬编码。
- `macaca-context` 不直连 Milvus、具体 MCP Server、具体 skill 源；`macaca-memory` 不碰 prompt/UI；skills 不负责 memory/MCP policy。
- 文件 ≤500 行；小步可逆变更；trait/adapter/value object 优先。
- Superpowers/OpenSpec 惯例：语义化行为前先更新 `openspec/changes/*/spec`，再改代码。
- GitNexus：改符号前先 `impact`，提交前 `detect_changes`（环境可用时必须执行）。

### 1.2 当前已实现（不必推倒重来）

- **Phase A 等价物**：`ContextCandidate`/`ContextProvider`/`ContextFacade`/`DefaultContextComposer`/`ContextPlan`/`ContextReport`、`assemble_context_providers` 配置驱动的 family catalog。
- **Phase B**：`ProfileFileContextProvider`、`AgentProfileFileKind`、loader 禁锢/截断、`MEMORY.md` audit 语义与诊断。
- **Phase C**：`MemoryActiveRecallContextProvider` + `ActiveRecallCapability` + `WorkspaceMemoryRecallSource` 的 AgentPrivate/session shared 路由 + 超时 fail-open。
- **Phase D（Skills）**：`SkillContextProvider` + compact catalog + 依赖缺口 diagnostics。
- **Phase D（MCP）**：`McpContextProvider` + fenced + untrusted + collision diagnostics（摘要级，不占 resource body）。
- **Phase 部分 digest/governance**：`KnowledgeDigestContextProvider`、`apply_digest_vs_raw_selection`、`TombstoneIndex`/`SharedTombstoneRegistry`、`filter_digest_items_by_tombstones`、web 侧 `WorkspaceMemoryForgetTool` 接线。
- **Phase 本地扩展边界**：`ContextProviderRegistry`/`ContextProviderFactory`、`OpaqueExternalPayload` 校验。

### 1.3 仍需「补全 Phase 字面要求」的差距（brainstorm）

| 主题 | 选项 | 利弊 |
|------|------|------|
| **Phase 0 单一 OpenSpec umbrella** | A) 新建 `integrate-context-memory-skills-mcp` meta-change（proposal+design 引用已有子 change + 归档策略） | 满足计划 ID；不费力气合并代码历史 |
| | B) 把所有 spec 揉成一条 mega delta | 审查成本高，易冲突 |
| **CompiledContext vs CompiledPrompt** | A) `pub type CompiledContext = CompiledPrompt` + 文档 | 低风险满足命名 |
| | B) 新 struct 包 `CompiledPrompt`+`ContextPlan` | 更清晰但波及面广 |
| **frontmatter stripping** | A) 仅删首个 `---\n...\n---` YAML 块（不解析 schema） | 简单、够用 |
| | B) full gray_matter crate | 多依赖 |
| **profile scanner hook** | A) `AgentProfileContextConfig` 可选 `Arc<dyn ProfileContentScanner>` 由 web/kernel 注入 | 真正 Hook；需 proto 或可配置回调桥 |
| | B) crate 内置「轻量正则/禁止指令」scanner | `macaca-context` 自持，耦合扫描规则 |
| **召回 tombstone（Cross-cutting）** | A) `MemoryRecallQuery`/`MemorySourceProvider` 链路透传 `Arc<dyn TombstoneIndex>`，`WorkspaceMemoryRecallSource` 过滤 | 对齐计划字面；不改 DB 语义 |
| | B) 仅依赖 `forget` 物理删除 | 与「仅存 tombstone 不删」的 governance 语义不一 |
| **stable prefix hash vs dynamic recall** | A) `ContextFacade`/`CompiledPrompt` 显式拆分 `stable_fingerprint` 与 `dynamic_fingerprint` 并写入 `ContextReport` | 可测 |
| | B) 仅文档说明 | 无法验收 cross-cutting |

**收敛（推荐）：**  
- Phase 0 走 **meta OpenSpec**。  
- `CompiledContext` 用 **type alias + rustdoc**。  
- frontmatter 用 **手写最小剥离**（无新依赖）。  
- scanner：**先 B（内置极简策略）**，预留 trait 钩子接口但不强制 proto 改动（第二场迭代再接 `ProfileContentScanner`）。  
- tombstone：**A**，与 digest 同源 `TombstoneIndex`。  
- stable/dynamic fingerprint：**A**，小字段进 `ContextReport` 或可复用 composer summary。

---

## 2. Write-Plan（按 Phase 落地的任务清单）

> 顺序：**OpenSpec → context-memory → web 接线 → tests → docs**。  
> 每个 PR 粒度：一个 Phase 或可独立验证的切面。

### Phase 0 — OpenSpec Planning（补「字面未完成」）

- [ ] **P0.1** 新建 `openspec/changes/integrate-context-memory-skills-mcp/`：`proposal.md`（为何要统一 façade；与已合并子 change 的关系）、`design.md`（umbrella：引用 `add-context-composer-foundation` 等到 `tasks.md` 映射表）。
- [ ] **P0.2** `tasks.md`：逐条勾选「计划 Phase 与现有 change id / 主干模块」对齐；明确 **遗留缺口由本 umbrella 的子任务接管**。
- [ ] **P0.3** Spec delta：**仅 ADDED** 「整合验收」requirements（composer 必选路径、召回 tombstone、tomb+digest、profile 安全加载、Capability 快照边界），每条含 Scenario。
- [ ] **P0.4** `openspec validate integrate-context-memory-skills-mcp --strict`。

**验收：** validate 严格通过；proposal 可被审查者映射到代码目录。

---

### Phase 1 — Composer 基础命名与收尾

- [ ] **P1.1** 在 `macaca-context`（如 `composer/mod.rs` 或 `prompt.rs`）导出 `pub type CompiledContext = CompiledPrompt`（或等价 bundle），rustdoc 指向 `ContextPlan` + merge 语义。
- [ ] **P1.2** （可选低风险）抽出 `LegacyNoopComposerProvider` 或在 `composer/provider.rs` 文档化「LegacyContextEngine ≈ noop engine path」，避免计划中 noop 无处可指。
- [ ] **P1.3** 单测：**stable hash**：对同一组 `ContextCandidate`，仅替换 `Dynamic` recall 正文时，`ComposerPlanSummary` / 新增 `stable_fingerprint` 不变（实现 P7.3 可先占位，最终指纹在 Phase 7 收口）。

**验收：** `cargo test -p macaca-context --lib`；不改默认模型可见输出的回归由现有 golden/集成测守护。

---

### Phase 2 — Agent Profile Provider 补全

- [ ] **P2.1** `profile/loader.rs`：在读入 UTF-8 文本后、`ProfileLoadOutput` 组装前剥离 **第一段**标准 YAML frontmatter（`^---\n` … `\n---\n`），剥离失败则整块保留并打 **diagnostic**（fail-open）。
- [ ] **P2.2** **内置极简 scanner**：例如最大行数/字节、禁止的子串占位表（可由 `AgentProfileContextConfig` 增字段或使用现有 `enabled/max_file_bytes` 组合）；命中则 `ProfileSkipReason` 或附带 `warn` diagnostics 进 candidate。
- [ ] **P2.3** `profile/tests.rs`：`inject_heartbeat = false` 时 **不得**产出 `HEARTBEAT` candidate；frontmatter 剥离用例。
- [ ] （可选）**P2.4** Proto：`inject_heartbeat` 默认改为 `false` **属行为变更** — 须在 umbrella spec 写明 Scenario；若保守则仅文档告知运维显式关闭。

**验收：** 新测试绿；openspec Scenario 对齐。

---

### Phase 3 — Active Recall × Tombstone

- [ ] **P3.1** `macaca-context`：`MemoryRecallQuery` 或 compose 上下文增加可选 **`tombstones: Option<Arc<dyn TombstoneIndex>>`**（或直接挂在 `WorkspaceMemoryRecallSource`/provider 构造函数，避免污染通用 query——**优选构造期注入**，不污染所有来源）。
- [ ] **P3.2** `WorkspaceMemoryRecallSource`：Recall 后对 entry id 字符串做 **`tombstoned_memory_id_strings` snapshot → HashSet → 过滤**；错误路径 **fail-open 并 tracing::warn**（与 digest 一致）。
- [ ] **P3.3** `macaca-web`：`ContextReportingChatModel`/recall capability 构造处传入 **与 digest/forget 相同**的 `Arc<SharedTombstoneRegistry>`（已存在则克隆 Arc）。
- [ ] **P3.4** `DefaultActiveRecallProvider` 是否需要过滤：**若snippet 仅从单一 source**，过滤在 source 一层足够；多条 source 则需统一后置 filter decorator（brainstorm：**优先 source 一层**，避免双层逻辑）。
- [ ] **P3.5** 单测：`macaca-context` recall fixture + mock `TombstoneIndex`，断言 snippet 不包含 tombstoned id；`cargo test -p macaca-web` 如有集成测则增补。

**验收：** Cross-cutting 「tombstoned memories never appear in recall candidates」在 **workspace** 路径成立。

---

### Phase 4 — Skills Capability（字面收尾）

- [ ] **P4.1** `pub type CapabilityCandidate = ContextCandidate` 或文档 §「CapabilityCandidate 即 `ContextCandidate { kind: CapabilityIndex }`」，放在 `composer/candidate.rs`。
- [ ] **P4.2** 回顾 `skill_provider`：**确认 SKILL.md body 未被 catalog 拉入**（已满足则只做 spec 勾选）。

---

### Phase 5 — MCP Capability（字面收尾）

- [ ] **P5.1** Spec：`McpContextProvider` **不传输 resource/prompt bodies**；仅 metadata + fence；碰撞 diagnostics 已有测试则引用 `provider_tests`。
- [ ] （可选）**P5.2** 为未来 `resource_uri` hints 预留 `McpCapabilityCatalog` 字段 — **仅当不超 500 行且有明确 consumer**。

---

### Phase 6 — Knowledge Digest & Governance（收尾）

- [ ] **P6.1** `GovernanceFacadeTombstones`：**若 governance 内存路径独立于 workspace forget**，文档化双线 tombstone merge 策略或在 facade 组装层 **Trait object 链**：`MergedTombstoneIndex(Vec<Arc<dyn TombstoneIndex>>)` 实现异步合并（按需）。
- [ ] **P6.2** 单测：digest + tombstone + stale raw 三路组合（可从现有 selection + tombstone_filter 拼装）。

---

### Phase 7 — Runtime / Web 全盘 Facade 与 fingerprint

- [ ] **P7.1** **Call-site 盘点**：rg `assemble\(|ContextRuntimeFacade::|LegacyContextEngine` 在非 `macaca-web`/测试路径的出现点；逐项标注是否必须经过 `ContextFacade`。
- [ ] **P7.2** 对仍为 legacy 的路径：要么挂 `ContextFacade::legacy()` + 注释「待迁」，要么开 issue 链接至 umbrella tasks.md（**不改行为**）。
- [ ] **P7.3** 实现 **`stable_fingerprint` / `dynamic_fingerprint`**（SHA256 或可复用 governance fingerprint 工具）：只对 **cache_class Stable** 候选参与 stable；dynamic recall 仅进 dynamic。**写入 `ContextReport` composer 小节**。
- [ ] **P7.4** SSE/event：`context_report` 已存在的字段补充 fingerprint（若体积敏感则用截断 hex）。

**验收：** 集成或单元测证明「dynamic recall 变 → stable fingerprint 不变」。

---

### Phase 8 — External Provider Boundary（字面收尾）

- [ ] **P8.1** `macaca-context` `README` 片段或 `governance/external_boundary.rs` 顶注：**in-process traits 稳定端口**；远程/WASM/MCP-provider **必须**经过 `OpaqueExternalPayload` + 候选转换。
- [ ] **P8.2** 示例：`#[cfg(test)]` 伪造 external adapter → `validate_opaque_external_payload` → `ContextCandidate`（证明 ACL 链路）。

---

## 3. 跨切面测试矩阵（须在 umbrella tasks 勾选）

| 断言 | 实现任务 |
|------|-----------|
| 动态 recall 不污染 canonical transcript | 已有注释 + CLI/集成回归；P7.1 盘点 |
| Profile 优先级 + heartbeat 门控 | P2.3 |
| MEMORY.md 非向量自动 ingest | 已有 diagnostics；spec 勾选 |
| Tombstone ∉ recall ∉ digest evidence | P3.* + tombstone_filter |
| Skill index 无全文 body | 已有 + P4.* |
| MCP fenced + collisions | `provider_tests` |
| Stable hash independence | P1.3 + P7.3 |

---

## 4. 验证命令（每 PR）

```bash
cd macaca && cargo check --workspace && cargo test -p macaca-context --lib && cargo test -p macaca-memory --lib && cargo test -p macaca-web --lib
npx openspec validate integrate-context-memory-skills-mcp --strict
# GitNexus（若 MCP 可用）：
# gitnexus_impact / gitnexus_detect_changes()
```

---

## 5. 建议 PR 拆分

1. **PR-0**：OpenSpec umbrella only。  
2. **PR-1**：Phase 1 alias + （可选）fingerprint schema 占位。  
3. **PR-2**：Phase 2 loader + scanner + tests。  
4. **PR-3**：Phase 3 tombstone recall + web 接线 + tests（最大行为面，单独审查）。  
5. **PR-4**：Phase 4–6 文档/类型别名 + governance merge（若需要）。  
6. **PR-5**：Phase 7 fingerprint + report + call-site checklist。  
7. **PR-6**：Phase 8 docs + 示例测试。

---

## 6. 风险与缓解

| 风险 | 缓解 |
|------|------|
| Proto 默认 heartbeat 语义变更惹争议 | P2 先测+文档；默认变更独立 PR + Spec Scenario |
| Tombstone双源（registry vs governance facade） | `MergedTombstoneIndex` 或明确「workspace 仅以 registry」的 spec |
| fingerprint 开销 | 只对入选 stable 候选字符串 hash；可缓存 planner tag |

---

