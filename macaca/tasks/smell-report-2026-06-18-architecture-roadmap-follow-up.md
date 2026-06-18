# 架构坏味道整改跟进报告

**项目：** Macaca Agent OS (`macaca/`)  
**日期：** 2026-06-18  
**关联提案：** `openspec/changes/refactor-architecture-smell-roadmap/`

## 本轮已处理

1. Shell 任务分解语义迁移到 Task Service fallback decomposition command/Strategy 路径，Web 仅作为 SDK/client 适配层创建返回的 Task Board assignments。
2. `serviceization_escape_hatches.rs` 从单个大型测试文件拆成 scanner、tokens、assertions、support、autonomy schedule policy 等支撑模块，保留原有 hard gate 语义。
3. 新增 450 行 OS 层文件大小 advisory gate，500 行 hard gate 不变。
4. 新增 shell semantic ownership gate，阻止 Web/CLI 重新拥有任务规划、分解、review 语义。
5. 拆分 `PaymentSystemServiceProvider` provider family：descriptor、support helpers、settlement state machine 与 command dispatch 分离，作为 provider extraction-readiness 参考样本。
6. 为 Skill MCP mapping hot path 增加 request-local `BTreeSet` 索引，保持 mapping 顺序和缺失行为。
7. 拆分 `macaca-proto` 的 `memory_service` DTO：scope、commands、results/snapshot 分离，并补 serde compatibility test。
8. 为现有静态 registry/lock 补充 lifecycle、restart、test-isolation 说明；高风险默认 registry 暂不替换。
9. 新增非失败 `architecture_smell_trend_gate`，输出确定性、脱敏、带 rule id 的趋势诊断。
10. 更新三份治理文档，明确 Task/Autonomy 服务语义所有权、provider extraction checklist、advisory diagnostics 规则。

## GitNexus 影响备忘

- `persist_session_snapshot`：GitNexus 标记 CRITICAL，影响 `post_chat_v2` 相关会话持久化流程。本轮未改变行为，只提取命名 helper 和补 lifecycle 注释。
- `default_skill_mcp_mapping_registry`：GitNexus 标记 HIGH，影响 skill/MCP resolution 和 framework toolkit 构建流程。本轮未替换静态 registry，只补 lifecycle 注释。
- `PaymentSystemServiceProvider`、`SkillMcpMappingEntry`、`memory_service.rs`、`serviceization_escape_hatches.rs`：影响面 LOW，本轮已做结构性拆分或局部索引。

## 剩余风险

- runtime-host 仍有多个 provider 文件处于 450 行以上，需要按同样 checklist 分批拆分。
- 部分 shell 文案仍使用“planner”等自然语言描述作为日志文本；当前 gate 阻止业务角色字面量作为结构化路由字段继续扩散。
- `architecture_smell_trend_gate` 是非失败 lane，只用于趋势可见性；真正阻断仍依赖 serviceization、dependency、shell ownership、file-size hard gates。
