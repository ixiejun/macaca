# Refactor macaca-llm Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 对 `macaca-llm` 做渐近式设计模式重构，先稳定 provider / model routing / wrapper 这些底层 contract，再让 framework、web、task、agent 等消费方逐步迁移。

**Architecture:** `macaca-llm` 是阶段 1 的底层 provider contract crate，仅依赖 `macaca-proto`。本轮计划遵循 additive-first：先把现有硬编码 prefix routing、provider construction、retry/rate-limit/cost 组合抽成可测试的 primitives，保持现有 public API 和行为 1:1，再为后续跨 crate 迁移提供稳定入口。

**Tech Stack:** Rust, `async_trait`, `reqwest`, `tokio`, `serde`, `serde_json`, `macaca-proto`, OpenSpec, GitNexus.

---

## Context

已阅读：

- `AGENTS.md`
- `openspec/AGENTS.md`
- `macaca/docs/design_patterns.md`
- `macaca/docs/design-pattern-refactor-plans/README.md`
- `macaca/docs/design-pattern-refactor-plans/refactor-order.md`
- `macaca/docs/design-pattern-refactor-plans/macaca-llm.md`
- `openspec/changes/refactor-llm-provider-model-routing/*`
- `macaca/crates/macaca-llm/src/lib.rs`
- `macaca/crates/macaca-llm/src/provider.rs`
- `macaca/crates/macaca-llm/src/router.rs`
- `macaca/crates/macaca-llm/src/openai.rs`
- `macaca/crates/macaca-llm/src/anthropic.rs`
- `macaca/crates/macaca-llm/src/dashscope.rs`
- `macaca/crates/macaca-llm/src/openai_compatible.rs`
- `macaca/crates/macaca-llm/src/resilient.rs`

当前代码事实：

- `LlmProvider` 是核心 provider trait，当前只包含 `name()` 和 `chat(...)`。
- `LlmRouter` 同时负责 provider registry、默认模型、model selection、fallback route plan、prefix provider resolution 和 `from_config` provider construction。
- `LlmRouter::resolve_provider_name` 当前用硬编码 prefix if/else 解析：
  - `model.contains('/')` -> `openrouter`
  - `gpt-*` / `o1*` / `o3*` -> `openai`
  - `claude-*` -> `anthropic`
  - `qwen*` -> `dashscope`
  - `deepseek-*` -> `deepseek`
  - case-insensitive `minimax-` -> `minimax`
  - unknown -> model string as provider key
- `LlmRouter::from_config` 当前直接 match provider name 构造 native provider 或 `OpenAiCompatibleProvider`，再包一层 `ResilientLlmWrapper`。
- `OpenAiProvider`、`DashScopeProvider`、`OpenAiCompatibleProvider` 里存在相似 OpenAI-compatible wire conversion 逻辑，但当前先不合并，以避免同时触碰协议行为。
- `ResilientLlmWrapper` 同时处理 retry/backoff、rate-limit、budget gate、cost tracking、fallback models，后续适合拆成 decorator chain。
- 文件行数现状：
  - `router.rs`: 659 行，超过项目 500 行规则。
  - `resilient.rs`: 619 行，超过项目 500 行规则。
  - 其余 provider 文件低于 500 行。

相关 OpenSpec：

- 已存在活跃 change：`refactor-llm-provider-model-routing`。
- 该 change 覆盖范围较大：`macaca-llm` + `macaca-framework` + `macaca-web` + 配置链路。
- 本计划建议先落一个 `macaca-llm` 内部重构切片，并让后续 OpenSpec 明确它与既有 `refactor-llm-provider-model-routing` 的关系，避免重复 proposal。

## Superpowers Brainstorm

### Option A: 只抽出 ProviderResolver 第一切片

做法：

- 新增 `resolver.rs`。
- 定义 `ProviderResolver` trait。
- 定义 `PrefixProviderResolver` 或 `StaticProviderResolver`，原样承载现有 prefix 规则。
- 定义 `ResolverChain`，按顺序匹配 provider。
- `LlmRouter::resolve_target` 改为调用 resolver chain。
- 保留 `LlmRouter::resolve_provider_name` 作为 deprecated 或 private compatibility wrapper，测试迁移到 resolver。

Benefits:

- 风险最低，行为最容易 1:1 验证。
- 直接解决 provider 选择硬编码问题。
- 能把 `router.rs` 拆小，满足文件行数约束的第一步。
- 为后续 provider registry、model route policy、framework adapter 打基础。

Risks:

- 只能解决 routing，不解决 provider construction 和 resilience wrapper 职责混杂。
- 如果 resolver API 设计过重，后续 factory registry 可能被迫兼容一个不够自然的接口。
- 需要谨慎保持 unknown model fallback 行为：unknown provider 必须仍然回退到 model string 本身。

Conclusion:

- 推荐作为第一轮实际实施切片。

### Option B: 一次性完成 `macaca-llm` 内五个计划切片

做法：

- 抽 `ProviderResolver`。
- 拆 `ResilientLlmWrapper` 为 retry、timeout、rate-limit、cost decorators。
- 增加 `ConversationPolicy`。
- 增加 `LlmProviderFactory` / registry。
- 建立 provider contract tests。

Benefits:

- 与 `macaca/docs/design-pattern-refactor-plans/macaca-llm.md` 的完整目标最一致。
- 可以一次性把 `router.rs` 和 `resilient.rs` 的文件大小问题都解决。
- 后续 framework/web 迁移会更顺。

Risks:

- 单轮变更过大，不符合“小的、可审查的、可逆的变更”。
- 同时触碰 router、factory、decorator、provider wire policy，行为漂移风险高。
- Provider contract tests 可能需要 test server / mock transport 设计，容易把规划阶段变成测试框架重构。

Conclusion:

- 不建议一次性实施。适合作为总路线，但每次只执行其中一个切片。

### Option C: 先拆 `ResilientLlmWrapper` decorator chain

做法：

- 新增 `decorators/` 或 `resilience/` 模块。
- 把 retry/backoff、rate limit、budget gate、cost tracking 拆为独立 `LlmProvider` wrappers。
- `ResilientLlmWrapper` 保留为 facade/builder，内部组合 wrapper chain。

Benefits:

- 直接降低 `resilient.rs` 文件大小和职责密度。
- 横切能力更符合 Decorator / Proxy。
- 后续可以按配置动态组合 retry、cost、rate-limit。

Risks:

- 当前 `ResilientLlmWrapper` 的 fallback model 与 retryable 判断交织较深，拆分时容易改变 retry/fallback 次序。
- `CostTracker` 记录时机、budget gate 时机和 rate limiter acquire 次数都需要严格保持。
- 相比 provider resolver，它的 blast radius 更难肉眼验证。

Conclusion:

- 推荐作为第二或第三切片，在 resolver 行为锁定后执行。

### Option D: 先引入 provider factory registry

做法：

- 新增 `factory.rs`。
- 定义 `LlmProviderFactory`，从 config provider entry 生成 `Arc<dyn LlmProvider>`。
- native provider 与 OpenAI-compatible provider 通过 factory registry 创建。
- `LlmRouter::from_config` 只负责遍历配置和注册 provider。

Benefits:

- 直接解决 provider 创建硬编码。
- 与 `refactor-llm-provider-model-routing` 的目标高度一致。
- 新增 OpenAI-compatible provider 不再改 web/bootstrap 或 router match。

Risks:

- 当前 `from_config` 还负责 normalize base URL、默认模型、resilience wrapper 组装，边界需要先拆清楚。
- 如果先做 factory，resolver prefix 规则仍然硬编码，provider id 与 model id 的语义仍不完整。
- 可能需要修改 config contract，触发更大 OpenSpec 范围。

Conclusion:

- 推荐作为 resolver 后的后续切片。

## Recommendation

采用 Option A 作为第一实施切片：先抽 `ProviderResolver` / `ResolverChain`，把 `LlmRouter` 中 provider prefix 选择逻辑迁移为 Chain of Responsibility + Strategy。

理由：

- 它是 `macaca-llm.md` 中第一切片。
- 行为可以用现有 router tests 1:1 覆盖。
- 变更主要限定在 `macaca-llm` 内部，避免提前扩大到 framework/web。
- 它为已有 OpenSpec `refactor-llm-provider-model-routing` 提供更稳定的底层前置条件。
- 它可以先从 `router.rs` 拆出 routing 规则，解决超过 500 行的文件约束。

不做：

- 不改 provider wire request/response。
- 不改 `LlmProvider` trait。
- 不改 `LlmConfig` 配置格式。
- 不拆 `ResilientLlmWrapper`。
- 不迁移 `macaca-framework` / `macaca-web`。
- 不引入新依赖。
- 不硬编码 application、workflow、agent name。

## Design Pattern Mapping

- **Chain of Responsibility:** 多个 resolver 依次尝试解析 provider，第一命中生效。
- **Strategy:** prefix resolver、explicit-provider resolver、fallback resolver 都可以作为可替换策略存在。
- **Factory Method / Abstract Factory:** 后续 provider factory registry 用于 provider 创建，本切片只预留边界，不实施。
- **Decorator / Proxy:** 后续拆 `ResilientLlmWrapper` 时使用，本切片只避免扩大范围。
- **Adapter:** 后续 provider-specific request/response adapter 处理 OpenAI-compatible 与特殊 provider 差异，本切片不触碰 wire 协议。

## Proposed OpenSpec Handling

优先复用或收窄现有 active change：`refactor-llm-provider-model-routing`。

建议：

- 若该 change 仍计划覆盖 framework/web，则在其 `tasks.md` 前置新增 `macaca-llm resolver primitives` 子任务。
- 若希望严格小步实施，则创建新 change：
  - `refactor-macaca-llm-provider-resolver`
  - affected spec: `macaca-llm-provider-routing`
  - 明确它是 `refactor-llm-provider-model-routing` 的底层前置切片。

本计划推荐新建小 change，因为既有 change 范围太大，容易把第一轮 resolver 重构和 framework/web 迁移绑在一起。

## Files

First slice expected files:

- Create: `openspec/changes/refactor-macaca-llm-provider-resolver/proposal.md`
- Create: `openspec/changes/refactor-macaca-llm-provider-resolver/design.md`
- Create: `openspec/changes/refactor-macaca-llm-provider-resolver/tasks.md`
- Create: `openspec/changes/refactor-macaca-llm-provider-resolver/specs/macaca-llm-provider-routing/spec.md`
- Create: `macaca/crates/macaca-llm/src/resolver.rs`
- Modify: `macaca/crates/macaca-llm/src/lib.rs`
- Modify: `macaca/crates/macaca-llm/src/router.rs`

Later slices:

- Create: `macaca/crates/macaca-llm/src/factory.rs`
- Create: `macaca/crates/macaca-llm/src/decorators.rs` or `macaca/crates/macaca-llm/src/resilience/`
- Create: `macaca/crates/macaca-llm/src/conversation_policy.rs`
- Create: provider contract test helpers after routing/factory boundaries are stable.

## Write Plan

### Task 1: OpenSpec alignment

- [ ] 1.1 Read current `refactor-llm-provider-model-routing` proposal/design/tasks/spec.
- [ ] 1.2 Decide whether to amend that change or create `refactor-macaca-llm-provider-resolver`.
- [ ] 1.3 Create or update OpenSpec artifacts before code changes.
- [ ] 1.4 Validate with:

```bash
openspec validate refactor-macaca-llm-provider-resolver --strict
```

Expected:

```text
Change 'refactor-macaca-llm-provider-resolver' is valid
```

If amending the existing change instead:

```bash
openspec validate refactor-llm-provider-model-routing --strict
```

### Task 2: Impact and baseline

- [ ] 2.1 Refresh GitNexus if index is stale.
- [ ] 2.2 Run GitNexus impact for `LlmRouter`.
- [ ] 2.3 Run GitNexus impact for `resolve_target`.
- [ ] 2.4 Run GitNexus impact for `resolve_provider_name`.
- [ ] 2.5 Report direct callers, affected processes, and risk level before editing.
- [ ] 2.6 Run baseline tests:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-llm router -- --nocapture
cargo check -p macaca-llm
```

### Task 3: Add resolver primitives

- [ ] 3.1 Create `macaca/crates/macaca-llm/src/resolver.rs`.
- [ ] 3.2 Define `ProviderResolution` or use plain provider id string if enough for the first slice.
- [ ] 3.3 Define `ProviderResolver: Send + Sync`.
- [ ] 3.4 Define `PrefixProviderResolver` with current ordered rules.
- [ ] 3.5 Define `ResolverChain` with first-match semantics.
- [ ] 3.6 Add table-driven unit tests for current routing rules:
  - `openrouter` slash models
  - `openai` GPT / o-series
  - `anthropic` Claude
  - `dashscope` Qwen
  - `deepseek`
  - case-insensitive MiniMax
  - unknown model fallback to model string

### Task 4: Wire router to resolver

- [ ] 4.1 Add resolver field to `LlmRouter`.
- [ ] 4.2 Initialize default resolver chain in `LlmRouter::new`.
- [ ] 4.3 Change `resolve_target` to call resolver chain instead of local if/else.
- [ ] 4.4 Keep compatibility for existing tests and any internal calls.
- [ ] 4.5 Move provider resolution tests from `router.rs` to `resolver.rs`, leaving router tests focused on selection/dispatch.
- [ ] 4.6 Ensure `router.rs` drops below 500 code lines if practical in this slice.

### Task 5: Verification

- [ ] 5.1 Run format:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo fmt
```

- [ ] 5.2 Run focused tests:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-llm resolver -- --nocapture
cargo test -p macaca-llm router -- --nocapture
```

- [ ] 5.3 Run crate check:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo check -p macaca-llm
```

- [ ] 5.4 Run affected consumers check if public exports changed:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo check -p macaca-task -p macaca-agent -p macaca-kernel -p macaca-runtime -p macaca-web
```

- [ ] 5.5 Run OpenSpec strict validation.
- [ ] 5.6 Run `gitnexus_detect_changes(scope: "all")` before commit.

## Follow-up Slices

### Slice 2: Provider factory registry

- Move provider construction out of `LlmRouter::from_config`.
- Add `LlmProviderFactory`.
- Keep native provider construction and OpenAI-compatible fallback behavior unchanged.
- Update `from_config` tests for configured provider registration.

### Slice 3: Decorator chain for resilience

- Split retry/backoff, budget gate, rate limit, and cost tracking into wrappers.
- Keep `ResilientLlmWrapper` as compatibility facade or builder.
- Preserve call ordering:
  - budget check before call
  - rate limiter acquire once per full call
  - retry cycle per model
  - fallback only for retryable errors
  - cost record only on success

### Slice 4: Conversation policy

- Add provider-specific message policy abstraction.
- First policy: DeepSeek reasoning content round-trip constraints.
- Add regression test for `reasoning_content` preservation.

### Slice 5: Provider contract tests

- Build common test fixtures for chat response parsing, tool calls, usage, and thinking content.
- Avoid real network calls in unit tests.
- Add integration hooks only where env vars are explicitly present.

## Residual Risks

- Current GitNexus MCP tools are not exposed in this session; implementation phase must use available GitNexus tooling or document the fallback before editing symbols.
- `openspec list --specs` currently reports no baseline specs, so new delta specs must be scoped carefully to avoid duplicate capability naming.
- Existing active change `refactor-llm-provider-model-routing` overlaps with this plan; the implementation phase must choose one OpenSpec path before writing code.
