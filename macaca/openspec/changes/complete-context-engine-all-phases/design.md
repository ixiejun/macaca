## Context

`Macaca Context Engineering Brainstorm And Plan` 把上下文工程拆成 `Phase 0-10`。当前仓库已经具备：

- `macaca-context` 独立 crate 与基础 contract
- `ContextRuntimeFacade` / `ContextReportingChatModel`
- `PromptComposer` / `ContextFacade`
- `windowed` / `pruning` / `summary` engine
- `ContextReport` API / UI
- compaction summary / lineage persistence
- memory preflight / active recall 的一部分 runtime path

但这些能力还没有在“所有 Phases 最终完成”的意义上收口。当前剩余问题主要分五类：

1. Phase 状态与现实代码不一致，缺少最终收口 change
2. `Phase 6` 对原始 payload retrievability 的覆盖还不完整
3. `Phase 7` 有 API/events，但 lineage UI 仍不完整
4. `Phase 8` 缺少 wiki/digest runtime recall 闭环
5. `Phase 9-10` 的 plugin path 与 migration/archive discipline 未闭环

本设计更新自 `docs/superpowers/plans/2026-05-06-complete-context-engine-unfinished-phases-plan.md`。核心判断是：不要重写已有 context engine，而是围绕 `complete-context-engine-all-phases` 补齐 runtime、diagnostics、verification 闭环。

## Goals / Non-Goals

### Goals

- 为 `Phase 0-10` 提供一个统一的最终完成 OpenSpec。
- 以“contract + runtime + diagnostics + verification”四类证据定义完成标准。
- 明确 `Phase 6-10` 的剩余实现要求和验收场景。
- 统一收口 plugin path、external adapter safety、wiki/digest recall、lineage UI、migration discipline。
- 保持所有能力 application-generic，并通过 config/profile 选择行为。
- 保证 pruning、recall、compaction、external adapter 都不破坏 canonical transcript、EventLog 或 artifact store。
- 保持上下文系统可插拔，使用户可替换 context engine/provider，而不是和 Macaca Agent OS 强耦合。

### Non-Goals

- 不删除现有 legacy 兼容入口；只定义最终迁移纪律。
- 不要求在一个切片里同时重构所有无关基础设施。
- 不把上下文策略下沉进 `macaca-llm` 或上浮到应用专有代码。
- 不新增 crate；优先在现有 crate 内拆分模块。
- 不默认注入所有 memory、wiki、skill body 或 MCP resource body。
- 不把外部 adapter 变成默认必需依赖。

## Design Pattern Mapping

- **Facade**：`ContextFacade` / runtime-facing facade 是生产路径唯一推荐入口；旧 prompt helper 只能作为 compatibility adapter。
- **Adapter**：EventLog/session/artifact retrieval、lineage store、memory runtime、knowledge digest、external process/RPC/WASM provider 均通过 adapter 接入。
- **Strategy**：pruning render policy、source retrieval policy、lineage presentation policy、recall policy、provider selection、fallback policy 可替换。
- **Decorator**：redaction、tombstone、privacy filtering、trust fencing、timeout、schema validation、circuit breaker 作为 wrapper，而不是写进核心流程。
- **Repository**：canonical source payload retrieval 由 repository 抽象统一读取，UI 和 context engine 不直接拼底层 key。
- **Chain of Responsibility**：profile、memory、wiki/digest、skills、MCP、tool schema 继续作为 provider stages 进入 composer。
- **Memento**：compaction summary、successor segment、lineage root/tip 是可审计快照，不原地改写历史。
- **Ports and Adapters / Bridge**：Macaca 定义 context engine/provider ports，用户实现本地或外部 adapter；runtime selection 与具体实现解耦。

## Completion Model

每个 Phase 必须同时满足四类证据：

- Contract: trait、value object、config、OpenSpec requirement 存在且语义明确。
- Runtime: framework/runtime/web 支持路径实际使用该能力。
- Diagnostics: EventLog、API、UI 或 context report 能解释该能力的行为。
- Verification: 单测、集成测试、E2E、OpenSpec validation、GitNexus evidence 覆盖该能力。

Phase 只有四项都满足时才能标记 complete。

## Decision 1: 用“最终收口 change”而不是重写既有 change

### 选择

新增 `complete-context-engine-all-phases`，作为：

- 已有 context-engine changes 的总收口层
- `Phase 0-10` 的最终完成标准来源
- 后续实施与归档的总 checklist

### 原因

- 现有 change 已经有明确边界，直接重写会混淆“已完成”和“待完成”。
- 总收口 change 更适合表达跨多个现有 change 的剩余缺口与最终验收。

### 备选方案

- 直接继续扩写 `complete-context-engine-runtime-phases`
  - 缺点：会把原本偏 `Phase 0-5` 的 change 膨胀成跨全部阶段的大杂烩，难以审计。

## Decision 2: 用统一的 Phase Completion Matrix 作为交付门禁

每个 Phase 必须同时满足：

- Contract
- Runtime
- Diagnostics
- Verification

只有四项都满足，才能标记为 complete，并允许后续归档相关 change。

### 原因

- 当前最大问题之一就是 contract 存在，但 runtime / UI / tests 没完全到位。
- 统一矩阵可以防止“代码写了但还没真正接入”的假完成。

## Decision 3: Phase 6 以“原始数据可追溯”作为完成门槛

`Phase 6` 不只要求把大输出裁成 excerpt，更要求：

- tool result
- trace event
- command stdout/stderr
- file read
- search result
- skill / capability derived large payload

这些被裁剪 source 的原始 payload 必须仍然保存在 canonical store 中，并能通过 source ref 或受控 debug path 取回。

### 原因

- 这是 non-destructive pruning 的核心约束。
- 如果原文不能取回，pruning 就退化成 destructive summarization。

### 架构

- 定义或收敛 `ContextSourceArtifactRepository` 风格边界，提供 `resolve(ref, scope)` 和 `preview(ref, policy)` 能力。
- 每个 source kind 使用 Adapter 把已有 EventLog/session/artifact key 转成统一 retrieval ref。
- Retrieval policy 作为 Strategy，明确 `available`、`preview_only`、`forbidden`、`unavailable`。
- UI 只消费 API 返回的 bounded preview 和 unavailable reason，不直接读取底层 store。

### 风险控制

- 所有 ref 解析必须校验 app/session/agent scope，拒绝 cross-session access。
- 默认 API 返回 bounded preview；完整原文需要 debug/authorized mode。
- 如果旧 source 没有稳定原文位置，必须在 report 中记录 explicit unavailable reason，不能假装可取回。

## Decision 4: Phase 7 需要“真正的 lineage UI”，不只 API 与事件

`Phase 7` 的最终完成标准必须包含：

- manual compact API
- automatic compaction trigger
- lineage tip resolution
- root-to-tip lineage query
- trace / context report 中的 compaction diagnostics
- 前端 lineage 视图或等价交互入口

### 原因

- 当前代码已经有 route 和 persistence，但前端主要还是事件文本提示。
- 计划原文要求“Present one logical session in UI while retaining internal lineage”。

### 架构

- Normal UX 使用 logical session facade，默认 resolve 到 lineage tip。
- Debug UX 使用 lineage API 展开 root-to-tip chain。
- Compaction summary 使用 Memento 语义保存，不替换原始 transcript。
- UI 仅展示 root id、tip id、successor chain、summary metadata 和事件，不把 lineage 细节渗透到应用业务逻辑。

### 风险控制

- 左侧 session list 仍展示 logical session，避免 compaction successor 变成重复 session。
- Lineage panel 是 diagnostics，不改变默认消息流语义。
- 自动 compaction 必须有 context report/event 诊断，便于回溯。

## Decision 5: Phase 8 把 wiki/digest 视为与 memory recall 同级的 runtime source

`Phase 8` 的最终完成必须显式覆盖：

- `memory_search`
- `memory_get`
- wiki/digest recall entry
- preflight recall
- active vector recall

且这些注入都必须：

- 是 dynamic
- 是 untrusted
- 是 request-only
- 带 provenance / confidence / privacy tier
- 不写回 canonical transcript

### 原因

- 当前系统在 memory recall 上走得比 wiki/digest 更远，导致 Phase 8 不对称。
- 研究计划明确要求 wiki/digest 也通过 source provider contract 进入 runtime。

### 架构

- 优先以 `KnowledgeDigestContextProvider` / context source provider 作为 wiki/digest runtime entry point。
- read-only tools 如 `wiki_digest_search` / `knowledge_digest_get` 可作为后续补充，但不能绕过 context report。
- Recall output 始终是 dynamic/untrusted/request-only candidate。
- `digest-vs-raw selection` 作为 Strategy，避免 raw memory 与 digest 重复注入。
- Tombstone/redaction/privacy filtering 作为 Decorator 包裹 recall source。

### Request-Only Boundary

- Context providers 不接收 canonical transcript 的可变引用。
- Recall body 只存在于 assembled request / context report summary，不写回 session transcript。
- EventLog 可以记录 diagnostics 和 source metadata，但不得重复持久化 injected recall full body，除非明确 debug/artifact policy 允许。

## Decision 6: Phase 9 先完成 local custom engine，再闭环 external adapter

`Phase 9` 分两层：

1. local in-process custom engine/provider
2. process/RPC/WASM external adapter

最终完成标准要求：

- local custom engine 能被注册、选择、回退、测试
- external adapter path 有稳定的安全边界和运行时 fallback
- 任一失败不能拖垮主 loop

### 原因

- 这符合原始计划“先本地 trait，再外部 adapter”的顺序。
- 同时也能满足“所有 phases 全部完成”的要求。

### 架构

- In-process engine/provider 是稳定端口，需通过 conformance tests。
- Runtime boot 通过 Abstract Factory 从 system/app/agent profile config 构建 engine/provider family。
- External adapter 是可选 Bridge，不是默认依赖。
- External output 必须进入 anti-corruption layer：schema validation、max payload、trust fencing、timeout、circuit breaker、fallback。

### 风险控制

- 不一次性冻结所有 process/RPC/WASM 协议细节；先定义最小 adapter seam 与 safety contract。
- 外部 adapter 默认 experimental/off，只有显式配置才启用。
- 失败必须 fallback 到 configured fallback engine，不能 crash main loop。

## Decision 7: Phase 10 以“迁移清零 + 归档门禁”收尾

`Phase 10` 的交付不只是 deprecated 注解，而是：

- 旧入口保留但 searchable
- 新生产代码禁止继续调用旧入口
- `rg` 无新增 deprecated 生产调用
- OpenSpec tasks 全部完成
- `openspec validate --strict` 通过
- GitNexus impact / change detection 有记录
- 相关 change 具备归档条件

### 架构

- Compatibility API 保留在原模块，但标记 deprecated 或 rustdoc replacement。
- Production path 只允许依赖 facade/composer/runtime selection。
- 迁移检查用 audit tests / `rg` 脚本固化，避免后续回归。

### 风险控制

- 如果 `#[deprecated]` 会导致 warning policy 破坏构建，内部兼容入口至少必须有 rustdoc replacement 与 production audit test。
- archive 只能在 tasks、spec、tests、GitNexus evidence 都一致后执行。

## Risks / Trade-offs

- Risk: 总收口 change 过大
  - Mitigation: 在 `tasks.md` 中严格按 `Phase 6-10` 切片实施，并把 `Phase 0-5` 只作为 closure / verification 项

- Risk: 为了“完成所有 phases”而过度设计 external adapter
  - Mitigation: 先定义最小可运行 adapter 契约和安全要求，再按 transport family 扩展

- Risk: UI 完善导致前端范围变大
  - Mitigation: 只要求 lineage / context diagnostics 必要 UI，不附带无关 redesign

- Risk: 多个 active changes 之间边界不清
  - Mitigation: 在 tasks 中显式列出依赖的 existing changes 和归档顺序

- Risk: pruned source retrieval 泄漏敏感数据
  - Mitigation: retrieval repository 默认 bounded preview；full payload 需要 scope validation 与 debug/authorized path

- Risk: wiki/digest 与 raw recall 重复注入
  - Mitigation: 复用 digest-vs-raw selection strategy，并在 ContextReport 中展示 suppressed/skipped reason

- Risk: external adapter 过早绑定具体协议
  - Mitigation: local trait 先稳定；external adapter 只定义 minimal seam 和 safety requirements

## Migration Plan

1. 对齐 OpenSpec 与 phase status matrix。
2. 收口 `Phase 6` pruning retrievability 与 diagnostics。
3. 收口 `Phase 7` lineage UI 与 logical session UX。
4. 收口 `Phase 8` wiki/digest recall runtime path。
5. 收口 `Phase 9` custom engine/provider 与 external adapter seam。
6. 收口 `Phase 10` deprecated migration discipline。
7. 统一运行 Rust/frontend/OpenSpec/GitNexus 验证。
8. 仅在所有证据齐备后准备归档相关 changes。
