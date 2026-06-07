# Migrate macaca-llm Consumers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 迁移上层代码到本次基于设计模式重构后的 `macaca-llm` provider resolver / router / model selection primitives，避免上层继续分散实现 provider/model 选择、单 provider 假设和 fallback 语义。

**Architecture:** 保留 `LlmProvider` 作为底层执行 trait。迁移目标不是把所有 `LlmProvider` 调用都替换掉，而是把 provider/model resolution、route plan、framework `ChatModel` adapter 和 web/bootstrap 的模型选择统一到 `LlmRouter` / `ModelSelection` / resolver primitives。上层只在需要“选择模型或 provider”时依赖 router；执行层仍可依赖 `LlmProvider` trait。

**Tech Stack:** Rust, `async_trait`, `macaca-llm`, `macaca-framework`, `macaca-web`, `macaca-app`, `macaca-task`, `macaca-runtime`, `macaca-kernel`, OpenSpec, GitNexus.

---

## 1. Current Code Facts

已阅读：

- `AGENTS.md`
- `openspec/AGENTS.md`
- `macaca/docs/design_patterns.md`
- `docs/superpowers/plans/2026-05-03-refactor-macaca-llm.md`
- `openspec/changes/refactor-macaca-llm-provider-resolver/*`
- `openspec/changes/refactor-llm-provider-model-routing/*`
- `macaca/crates/macaca-llm/src/lib.rs`
- `macaca/crates/macaca-llm/src/resolver.rs`
- `macaca/crates/macaca-llm/src/router.rs`
- `macaca/crates/macaca-llm/src/router_tests.rs`
- `macaca/crates/macaca-framework/src/adapter.rs`
- `macaca/crates/macaca-framework/src/react_agent.rs`
- `macaca/crates/macaca-framework/src/model_impls.rs`
- `macaca/crates/macaca-web/src/lib.rs`
- `macaca/crates/macaca-web/src/state.rs`
- `macaca/crates/macaca-web/src/framework_runner.rs`
- `macaca/crates/macaca-app/src/llm_proxy.rs`
- `macaca/crates/macaca-app/src/workflow.rs`
- `macaca/crates/macaca-task/src/decompose.rs`
- `macaca/crates/macaca-task/src/plan_loop.rs`
- `macaca/crates/macaca-runtime/src/agentic_loop.rs`
- `macaca/crates/macaca-agent/src/agent.rs`
- `macaca/crates/macaca-agent/src/basic.rs`
- `macaca/crates/macaca-kernel/src/kernel.rs`
- `macaca/crates/macaca-sdk/src/builder.rs`
- `macaca/crates/macaca-integration-tests/tests/live_llm_test.rs`

当前重构事实：

- `macaca-llm` 已新增 resolver primitives：
  - `ProviderResolver`
  - `PrefixProviderResolver`
  - `ResolverChain`
- `LlmRouter::resolve_target` 已通过 resolver chain 解析 provider。
- `LlmRouter::resolve_provider_name` 保留但 deprecated，只应作为迁移期 grep 入口。
- `router.rs` 已把测试拆出到 `router_tests.rs`，降到 500 行以内。
- `resilient.rs` 仍是既有 619 行，属于后续 decorator 切片，不应绑进本轮消费者迁移。

上层消费事实：

- `macaca-web::start_server` 已经使用 `LlmRouter::from_config(&config.llm)`，并把 router 同时作为 `Arc<dyn LlmProvider>` 注入 `Kernel`。
- `AppState` 同时持有：
  - `llm: Arc<dyn LlmProvider>`
  - `llm_router: Arc<LlmRouter>`
  - `config.default_model`
- `macaca-framework::adapter` 已有两个 adapter：
  - `LlmProviderAdapter`: 单 provider -> `ChatModel`
  - `RoutedLlmAdapter`: router + pre-resolved `ModelSelection` -> `ChatModel`
- `macaca-web::framework_runner` 已经通过 `state.llm_router.resolve_selection(ModelSelectionRequest { agent_model, app_model, app_provider, system_model, .. })` 构建 `ModelSelection`，并用 `RoutedLlmAdapter` 构建 `ReActAgent`。
- `macaca-web` 仍保留 `AppState.llm`，用于 kernel / legacy runner / other direct paths。
- `macaca-app::LlmProxy` 仍独立实现 user override > app default > agent model 的字符串模型选择，且忽略 `provider` override 字段。
- `macaca-task::LlmDecomposer` 和 `PlanLoop::GoalEvaluator` 直接持有 `Arc<dyn LlmProvider>` + model string。`GoalEvaluator::new/evaluate` 已被 deprecated，推荐 framework path。
- `macaca-agent`、`macaca-runtime`、`macaca-kernel`、`macaca-sdk` 多数地方只依赖 `LlmProvider` trait 做执行，属于合理底层边界，不应强行迁移。
- `macaca-integration-tests::live_llm_test` 已直接使用 `LlmRouter::from_config`，适合作为路由兼容验收。

OpenSpec 状态：

- `refactor-macaca-llm-provider-resolver` 已存在并完成，代表底层 resolver 第一切片。
- `refactor-llm-provider-model-routing` 已存在但 0/13 tasks，范围正好覆盖上层 provider/model routing 迁移。
- `openspec list --specs` 当前仍显示 no specs found，因此消费者迁移应优先更新既有 active change，而不是创建重复能力名。

## 2. Superpowers Brainstorm

### Option A: 只迁移 framework/web routed model path

做法：

- 复用 `refactor-llm-provider-model-routing`。
- 审查并补齐 `RoutedLlmAdapter` 与 `FrameworkRunner::resolve_model_selection`。
- 把 framework runner 中仍存在的单模型字符串假设收敛为 `ModelSelection` / `ModelTarget`。
- 保持 `AppState.llm` 和 legacy `LlmProvider` 执行入口不动。
- 增加 focused tests 覆盖 agent override、app default、provider-qualified model、fallback chain。

Benefits:

- 风险最低，切片聚焦真正运行链路。
- 与现有 `refactor-llm-provider-model-routing` OpenSpec 对齐。
- 不触碰 task/runtime/kernel 的稳定 `LlmProvider` trait 边界。
- 可快速验证 fullstack-autodev coordinator / planner / worker 的 routed model 命中。

Risks:

- `macaca-app::LlmProxy` 仍保留自有模型选择逻辑。
- 非 framework legacy paths 仍可能只看 `Arc<dyn LlmProvider>`，但这是兼容需要。
- 如果 tests 不覆盖 provider override，可能遗漏 `provider` 字段被忽略的问题。

Conclusion:

- 推荐作为第一轮消费者迁移切片。

### Option B: 将 `macaca-app::LlmProxy` 迁移到 `LlmRouter::resolve_selection`

做法：

- 新增 router-backed proxy 或把 `LlmProxy` 构造改为接收 `Arc<LlmRouter>`。
- 用户 override、app default、agent model 统一进入 `ModelSelectionRequest`。
- `provider` override 不再被忽略，而是作为 provider hint 或 provider-qualified target。
- 旧 `LlmProxy::new(inner, app_defaults, user_overrides)` 标记 deprecated 并保留。

Benefits:

- 直接消除 app 层重复模型优先级逻辑。
- 让 provider override 字段真正生效。
- 为 sdk/app/web 的声明式 LLM 配置统一语义。

Risks:

- `macaca-app` 目前依赖的是 trait provider，改成 router 可能扩大 API 面。
- 需要厘清 user override 的 provider-only 情况：无 model 时应使用 provider default model 还是 app model。
- 可能影响已有 app runtime tests。

Conclusion:

- 适合作为第二切片，不应和 framework/web 第一切片混在一起。

### Option C: 将 task/runtime/kernel 全部迁移为 router-aware

做法：

- `LlmDecomposer`、`GoalEvaluator`、`AgenticLoop`、`Kernel`、`DeclarativeAgent` 全部接收 router 或 `ModelSelection`。
- `LlmOptions.model` 不再由各处字符串保存。

Benefits:

- 从表面上看可以彻底统一模型选择。
- 后续 trace 中更容易记录 provider/model route plan。

Risks:

- 过度迁移，违背保留 `LlmProvider` trait 作为执行边界的设计。
- 会触碰大量通用基础设施和测试 mock，变更面大。
- `AgenticLoop` 和 `Agent` 应该只执行模型调用，不应该决定 provider routing。
- 容易把 bottom-layer router 语义反向污染所有上层 API。

Conclusion:

- 不推荐。只迁移真正做模型/provider 选择的模块。

### Option D: 先做 provider factory registry，再迁移消费者

做法：

- 回到 `macaca-llm`，先把 `LlmRouter::from_config` provider construction 抽成 factory registry。
- 再让 web/app/framework 消费更稳定的 factory/router。

Benefits:

- 底层 provider 创建更干净。
- 后续新增 provider 更少改动。

Risks:

- 当前用户目标是迁移上层消费者；继续底层 refactor 会延迟消费方收口。
- factory registry 和上层 routed model 是相关但不同切片。
- `from_config` 当前已能作为消费者迁移的可用 facade。

Conclusion:

- 可作为后续底层切片，不作为本轮消费者迁移前置。

## 3. Recommendation

采用 Option A 作为第一轮消费者迁移：聚焦 `macaca-framework` + `macaca-web` routed model path，复用并收窄现有 OpenSpec `refactor-llm-provider-model-routing`。

设计模式对应：

- **Facade:** `LlmRouter::from_config` 和 `resolve_selection` 作为上层 model routing facade。
- **Adapter:** `RoutedLlmAdapter` 把 `LlmRouter` / `ModelSelection` 适配到 framework `ChatModel`。
- **Strategy / Chain of Responsibility:** provider inference 已由 `ResolverChain` 承担，上层不再写 prefix 规则。
- **Builder:** `ModelSelectionRequest` 目前是 struct literal，后续可补 builder，但本轮不强加新抽象。
- **Decorator / Proxy:** `LlmProxy` 后续迁移用 proxy 模式保留兼容接口，本轮只规划，不实施。

本轮不做：

- 不删除 `LlmProvider`。
- 不把 `AgenticLoop` / `Kernel` / `Agent` 改成强依赖 router。
- 不改 provider wire protocol。
- 不拆 `ResilientLlmWrapper`。
- 不引入新依赖。
- 不硬编码 app name、workflow name、provider-specific application logic。

## 4. Proposed OpenSpec Handling

复用 existing change：

```text
refactor-llm-provider-model-routing
```

原因：

- 它的 proposal/design 已经准确描述上层 provider/model routing 迁移目标。
- tasks 目前 0/13，适合把已完成的底层 resolver 前置条件标入，并把后续任务细化为小切片。
- 新建 `migrate-macaca-llm-consumers-*` 会和现有 change 重叠。

建议在该 change 中更新：

- `design.md`
  - 记录 `refactor-macaca-llm-provider-resolver` 已完成，是底层前置切片。
  - 明确 `LlmProvider` 仍是执行 trait，不作为 deprecated target。
  - 明确消费者迁移只针对 routing/model-selection ownership。
- `tasks.md`
  - 将底层 resolver 前置任务标为完成或引用已完成 change。
  - 第一实施切片改为 framework/web routed path 审查和测试。
  - 第二实施切片再处理 `macaca-app::LlmProxy`。
- `specs/llm-provider-model-routing/spec.md`
  - 补充“legacy `LlmProvider` execution boundary remains valid”或“routing decisions SHALL be centralized when a caller chooses provider/model”。

## 5. Files for First Implementation Slice

Likely modify:

- `openspec/changes/refactor-llm-provider-model-routing/design.md`
- `openspec/changes/refactor-llm-provider-model-routing/tasks.md`
- `openspec/changes/refactor-llm-provider-model-routing/specs/llm-provider-model-routing/spec.md`
- `macaca/crates/macaca-framework/src/adapter.rs`
- `macaca/crates/macaca-web/src/framework_runner.rs`
- tests near `macaca-framework` / `macaca-web` if existing harness supports focused assertions

Potential later slice:

- `macaca/crates/macaca-app/src/llm_proxy.rs`
- `macaca/crates/macaca-app/src/workflow.rs`
- related app tests

Avoid touching in first slice unless tests show a direct need:

- `macaca/crates/macaca-agent/*`
- `macaca/crates/macaca-runtime/src/agentic_loop.rs`
- `macaca/crates/macaca-kernel/src/kernel.rs`
- `macaca/crates/macaca-sdk/src/builder.rs`
- `macaca/crates/macaca-task/src/decompose.rs`
- `macaca/crates/macaca-task/src/plan_loop.rs`

## 6. Write Plan

### Task 1: OpenSpec update

- [ ] 1.1 Read current `refactor-llm-provider-model-routing` proposal/design/tasks/spec.
- [ ] 1.2 Update design to reference completed `refactor-macaca-llm-provider-resolver`.
- [ ] 1.3 Update tasks into sequential migration slices:
  - prerequisite resolver complete
  - framework/web routed path
  - app proxy follow-up
  - compatibility and verification
- [ ] 1.4 Update spec delta to clarify routing ownership and `LlmProvider` compatibility.
- [ ] 1.5 Validate:

```bash
openspec validate refactor-llm-provider-model-routing --strict
```

### Task 2: Impact and baseline

- [ ] 2.1 Run GitNexus impact before editing:

```bash
npx gitnexus impact RoutedLlmAdapter --repo agent --direction upstream
npx gitnexus impact resolve_model_selection --repo agent --direction upstream
npx gitnexus impact build_react_agent --repo agent --direction upstream
npx gitnexus impact LlmProviderAdapter --repo agent --direction upstream
```

- [ ] 2.2 Report direct callers, affected processes, and risk levels.
- [ ] 2.3 Run baseline checks:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-llm router -- --nocapture
cargo test -p macaca-framework adapter -- --nocapture
cargo check -p macaca-framework -p macaca-web
```

### Task 3: Framework adapter migration

- [ ] 3.1 Review `RoutedLlmAdapter` routing behavior:
  - no explicit model -> use pre-resolved `ModelSelection`
  - explicit model override -> route through `LlmRouter::chat`
  - default route -> use `chat_with_selection`
- [ ] 3.2 Add or update tests so `RoutedLlmAdapter` proves it uses `chat_with_selection` for default route and `chat` for explicit override.
- [ ] 3.3 Keep `LlmProviderAdapter` callable for legacy direct provider integration, but mark deprecated only if no active non-test production path requires it.
- [ ] 3.4 Do not remove `LlmProviderAdapter`.

### Task 4: Web framework runner migration

- [ ] 4.1 Review `FrameworkRunner::resolve_model_selection` precedence:
  - agent model
  - app `llm_config.model` + `llm_config.provider`
  - system default model
  - router default target
- [ ] 4.2 Add tests for:
  - agent override beats app/system
  - app provider hint resolves provider-qualified route
  - system default fallback works
  - fallback chain is preserved if configured or introduced through request
- [ ] 4.3 Ensure all framework agent construction paths call `RoutedLlmAdapter::new`.
- [ ] 4.4 Search and eliminate production framework/web usage of manual provider/model choice outside router:

```bash
rg -n "LlmProviderAdapter|resolve_provider_name|default_provider|providers\\.|llm_config|ModelSelectionRequest" macaca/crates/macaca-framework macaca/crates/macaca-web
```

### Task 5: Compatibility and non-migration checks

- [ ] 5.1 Confirm remaining direct `Arc<dyn LlmProvider>` usage is execution-only, not model/provider selection.
- [ ] 5.2 Keep task/runtime/kernel/agent/sdk trait boundaries unchanged unless a direct model-selection duplication is found.
- [ ] 5.3 Document `macaca-app::LlmProxy` as follow-up if not migrated in the first slice.
- [ ] 5.4 If migrating `LlmProxy`, first add router-backed constructor and deprecate the old constructor without deleting it.

### Task 6: Verification

- [ ] 6.1 Run format:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo fmt
```

- [ ] 6.2 Run focused tests:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-llm router -- --nocapture
cargo test -p macaca-framework adapter -- --nocapture
cargo test -p macaca-web framework_runner -- --nocapture
```

- [ ] 6.3 Run checks:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo check -p macaca-llm -p macaca-framework -p macaca-web
```

- [ ] 6.4 Run OpenSpec strict validation.
- [ ] 6.5 Run GitNexus detect changes:

```bash
npx gitnexus detect-changes --repo agent --scope all
```

## 7. Residual Risks

- Existing `refactor-macaca-llm-provider-resolver` changes are currently uncommitted in the working tree. Consumer migration implementation should either commit that first or clearly keep both changes in one review boundary.
- `macaca-web/src/framework_runner.rs` is large and broad; tests may be hard to isolate without adding helper seams. Any helper extraction must run GitNexus impact first.
- `macaca-app::LlmProxy` currently ignores provider-only overrides. Migrating it is valuable but should be a separate slice unless framework/web tests already require it.
- `LlmProviderAdapter` may still be useful for tests or non-router embeddings of framework. Deprecating it too early could create churn without improving routing ownership.
