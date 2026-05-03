# Migrate macaca-memory Consumers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 迁移上层代码到 `macaca-memory` 本次基于设计模式重构后的 facade / request / result / backend primitives，停止上层继续调用旧的 store/retrieve 语义入口。

**Architecture:** 以 `macaca-memory` 作为记忆能力的 canonical contract，`macaca-agent` 暴露面向 agent 的记忆服务 facade，`macaca-kernel` 通过 adapter 把具体 memory backend 接到 agent services。迁移保持 additive-first：旧接口保留并标记 deprecated，上层新代码只走新入口。

**Tech Stack:** Rust workspace, `async_trait`, `macaca-memory`, `macaca-agent`, `macaca-kernel`, OpenSpec, GitNexus.

---

## 1. 当前代码事实

已阅读：

- `macaca/crates/macaca-memory/src/facade.rs`
- `macaca/crates/macaca-memory/src/manager.rs`
- `macaca/crates/macaca-memory/src/isolated.rs`
- `macaca/crates/macaca-memory/src/backend.rs`
- `macaca/crates/macaca-memory/src/query.rs`
- `macaca/crates/macaca-memory/src/snapshot.rs`
- `macaca/crates/macaca-agent/src/services.rs`
- `macaca/crates/macaca-kernel/src/services.rs`
- `macaca/crates/macaca-framework/src/memory.rs`
- `openspec/changes/refactor-macaca-memory-patterns/*`
- `macaca/docs/design-pattern-refactor-plans/macaca-memory.md`
- `macaca/docs/design-pattern-refactor-plans/refactor-order.md`

实际上层消费者：

- `macaca-kernel` 是当前唯一直接依赖 `macaca-memory` 的上层生产代码。
- `macaca-agent` 当前没有依赖 `macaca-memory`，但它定义了 `MemoryService` 抽象，仍使用 `store(MemoryEntry)` / `retrieve(&str, usize)` 旧语义。
- `macaca-kernel::MemoryServiceAdapter<S: MemoryStore>` 仍把任意 `MemoryStore` 直接适配为 `macaca-agent::MemoryService`。
- `macaca-integration-tests` 依赖 `macaca-memory`，但当前扫描未发现直接生产消费路径。
- `macaca-framework::memory` 是 framework 自己的 working/long-term memory 模块，不直接依赖 `macaca-memory`；本轮不应把 framework working memory 与 persistent memory 强行合并。

当前问题：

- `macaca-memory` 已有 `RememberText` / `RecallQuery` / `RecallResult` / `ForgetMemory`，但上层 `MemoryService` 仍停留在 raw `MemoryEntry` 和 raw query string。
- 上层调用无法表达 facade contract，也无法自然接入后续 backend factory、snapshot、query strategy。
- 旧接口尚未在 `macaca-agent` 层 deprecated，后续代码仍可能继续写回旧模式。

## 2. 可选方案与风险

### 方案 A：只迁移 `macaca-kernel::MemoryServiceAdapter`

做法：

- 保持 `macaca-agent::MemoryService` 不变。
- 在 kernel adapter 内部用 `RememberText` / `RecallQuery` 包一层，再转回旧 trait。

优点：

- 变更最小。
- 不新增 `macaca-agent -> macaca-memory` 依赖。

风险：

- 上层 canonical contract 仍然是 `store/retrieve`。
- deprecated 只能标在 adapter 内部，无法阻止 agent service 调用侧继续使用旧入口。
- 不满足“迁移上层代码到 macaca-memory 重构版本”的目标，只是局部包装。

结论：不推荐。

### 方案 B：把 `macaca-agent::MemoryService` 迁移为 facade-first

做法：

- 让 `macaca-agent` 依赖 `macaca-memory`。
- 在 `MemoryService` trait 增加 canonical 方法：
  - `remember_text(RememberText) -> MemoryId`
  - `recall(RecallQuery) -> RecallResult`
- 将旧 `store(MemoryEntry)` / `retrieve(&str, usize)` 标记 deprecated，并提供兼容默认实现或在所有内部 impl 中保留。
- `macaca-kernel::MemoryServiceAdapter` 改为适配 facade-capable backend。

优点：

- 上层 agent service 与 memory crate 的 facade contract 对齐。
- 旧接口可 grep、可编译期 warning、内部不再调用。
- 行为能 1:1 还原，迁移面很小。

风险：

- 新增 `macaca-agent -> macaca-memory` 依赖，需要确认不会形成循环。
- `macaca-agent` 的抽象层会直接暴露 memory crate 类型，但从 refactor-order 看 `macaca-memory` 是底层 contract，方向合理。

结论：推荐。

### 方案 C：在 `macaca-memory` 新增 object-safe service trait，再让 agent/kernel 都消费它

做法：

- 在 `macaca-memory` 新增 `MemoryFacadeService` trait。
- `MemoryManager` / `IsolatedMemoryManager` 实现该 trait。
- `macaca-agent::MemoryService` 直接复用或包装该 trait。

优点：

- facade 抽象完全归属于 `macaca-memory`，更符合基础设施分层。
- 后续 Milvus / hybrid search / snapshot backend 可以继续扩展同一个 trait。

风险：

- 比方案 B 多一层 trait 和 adapter，当前真实消费者很少，容易过度设计。
- 需要更谨慎处理 object safety、async_trait、泛型 manager 实现。

结论：可作为方案 B 的后续增强，不作为本轮第一迁移切片。

## 3. 推荐方案

采用方案 B：`macaca-agent::MemoryService` facade-first，`macaca-kernel` adapter 跟进迁移。

设计模式对应：

- **Facade:** `MemoryService` 面向 agent 暴露 `remember_text` / `recall`，隐藏 `MemoryEntry` 构造与多层存储细节。
- **Adapter:** `macaca-kernel::MemoryServiceAdapter` 负责把 `macaca-memory` concrete backend 接入 `macaca-agent` 服务接口。
- **Null Object:** `NoopMemoryService` 继续作为无 memory backend 的空实现，保证无服务场景不写入、不召回。
- **Builder:** `AgentServices::builder().memory(...)` 继续作为服务注入入口，不改已有服务组合方式。

不做：

- 不改 `macaca-framework::WorkingMemory` / `LongTermMemory` 行为。
- 不迁移 task/trace/session 行为。
- 不删除任何旧接口。
- 不引入应用名、agent 名、workflow 名等硬编码。
- 不把 `MemoryStore` / `EmbeddingProvider` / `VectorStore` 底层 trait 标记 deprecated；它们仍是 backend contract。

## 4. 预期 OpenSpec

建议 change id：

```text
migrate-memory-consumers-to-facade-primitives
```

OpenSpec 文件：

- `openspec/changes/migrate-memory-consumers-to-facade-primitives/proposal.md`
- `openspec/changes/migrate-memory-consumers-to-facade-primitives/design.md`
- `openspec/changes/migrate-memory-consumers-to-facade-primitives/tasks.md`
- `openspec/changes/migrate-memory-consumers-to-facade-primitives/specs/macaca-memory-consumer-migration/spec.md`

核心规范：

- 上层 agent memory service SHALL expose facade-first `remember_text` / `recall` APIs backed by `macaca-memory` request/result types.
- Deprecated compatibility methods SHALL remain callable but SHALL NOT be used by migrated upper-crate production code.
- Kernel adapter SHALL adapt memory backends through facade-first calls while preserving existing storage/retrieval behavior.
- No-op memory service SHALL preserve current behavior: no persistent write, no recall result.

## 5. 文件结构计划

修改：

- `macaca/crates/macaca-agent/Cargo.toml`
  - 添加 `macaca-memory = { workspace = true }`。
- `macaca/crates/macaca-agent/src/services.rs`
  - 引入 `RememberText` / `RecallQuery` / `RecallResult`。
  - 为 `MemoryService` 增加 facade-first 方法。
  - 标记旧 `store` / `retrieve` deprecated。
  - 更新 no-op 和测试 recording service。
- `macaca/crates/macaca-kernel/src/services.rs`
  - 把 `MemoryServiceAdapter` 从 `S: MemoryStore` 迁移为 facade-capable backend adapter。
  - 对 `MemoryManager` / `IsolatedMemoryManager` 或通用 facade backend 提供实现。
  - 内部测试从 `adapter.store/retrieve` 改为 `remember_text/recall`。
- `macaca/crates/macaca-kernel/src/lib.rs`
  - 若 adapter 类型名保持不变，则通常不改；若拆出新 adapter 类型，需要保持 re-export 兼容。

新增或修改测试：

- `macaca-agent`：
  - no-op facade 不写入且 recall 返回空。
  - builder 注入的 memory service 通过 facade 被调用。
  - deprecated 方法只在兼容性测试中出现，并用局部 `#[allow(deprecated)]` 包住。
- `macaca-kernel`：
  - adapter facade remember/recall 行为与旧 adapter store/retrieve 等价。
  - grep 验证上层生产代码不再调用 deprecated memory service 方法。

## 6. 分步实施计划

### Task 1: OpenSpec 产物

**Files:**

- Create: `openspec/changes/migrate-memory-consumers-to-facade-primitives/proposal.md`
- Create: `openspec/changes/migrate-memory-consumers-to-facade-primitives/design.md`
- Create: `openspec/changes/migrate-memory-consumers-to-facade-primitives/tasks.md`
- Create: `openspec/changes/migrate-memory-consumers-to-facade-primitives/specs/macaca-memory-consumer-migration/spec.md`

- [ ] Step 1: 确认 change id 未占用。

Run:

```bash
test ! -e openspec/changes/migrate-memory-consumers-to-facade-primitives
```

Expected: command exits 0.

- [ ] Step 2: 创建 OpenSpec proposal/design/tasks/spec。

Spec delta must include scenarios for:

- facade-first agent memory service
- deprecated compatibility methods remain callable
- kernel adapter preserves behavior
- no-op memory remains no-op

- [ ] Step 3: 校验 OpenSpec。

Run:

```bash
openspec validate migrate-memory-consumers-to-facade-primitives --strict
```

Expected:

```text
Change 'migrate-memory-consumers-to-facade-primitives' is valid
```

### Task 2: Impact 与基线

**Files:** none.

- [ ] Step 1: GitNexus impact for `MemoryService`.

Run through MCP:

```text
gitnexus_impact({ target: "MemoryService", direction: "upstream", repo: "agent" })
```

Expected: identify direct implementers and service-builder consumers.

- [ ] Step 2: GitNexus impact for `MemoryServiceAdapter`.

Run through MCP:

```text
gitnexus_impact({ target: "MemoryServiceAdapter", direction: "upstream", repo: "agent" })
```

Expected: identify kernel exports/tests and any SDK/web consumers.

- [ ] Step 3: Baseline compile.

Run:

```bash
cd macaca
cargo check -p macaca-agent -p macaca-kernel
```

Expected: compile succeeds before migration.

### Task 3: Migrate `macaca-agent::MemoryService`

**Files:**

- Modify: `macaca/crates/macaca-agent/Cargo.toml`
- Modify: `macaca/crates/macaca-agent/src/services.rs`
- Modify if needed: `macaca/crates/macaca-agent/src/lib.rs`

- [ ] Step 1: Add `macaca-memory` dependency to `macaca-agent/Cargo.toml`.

Expected dependency:

```toml
macaca-memory = { workspace = true }
```

- [ ] Step 2: Add facade-first trait methods.

Intended shape:

```rust
use macaca_memory::{RecallQuery, RecallResult, RememberText};

#[async_trait]
pub trait MemoryService: Send + Sync {
    async fn remember_text(&self, input: RememberText) -> MacacaResult<MemoryId>;

    async fn recall(&self, query: RecallQuery) -> MacacaResult<RecallResult>;

    #[deprecated(note = "use MemoryService::remember_text with macaca_memory::RememberText")]
    async fn store(&self, entry: MemoryEntry) -> MacacaResult<MemoryId> {
        let mut input =
            RememberText::new(entry.content)
                .layer(entry.layer)
                .metadata(entry.metadata);
        if let Some(agent_id) = entry.agent_id {
            input = input.agent_id(agent_id);
        }
        self.remember_text(input).await
    }

    #[deprecated(note = "use MemoryService::recall with macaca_memory::RecallQuery")]
    async fn retrieve(&self, query: &str, limit: usize) -> MacacaResult<Vec<MemoryEntry>> {
        Ok(self.recall(RecallQuery::new(query, limit)).await?.entries)
    }
}
```

- [ ] Step 3: Update `NoopMemoryService`.

Expected behavior:

```rust
async fn remember_text(&self, _input: RememberText) -> MacacaResult<MemoryId> {
    Ok(MemoryId::new())
}

async fn recall(&self, _query: RecallQuery) -> MacacaResult<RecallResult> {
    Ok(RecallResult::new(Vec::new()))
}
```

- [ ] Step 4: Update tests to use facade methods.

Replace production-style test calls:

```rust
services.memory_service().retrieve("noop", 5).await.unwrap()
```

with:

```rust
services
    .memory_service()
    .recall(macaca_memory::RecallQuery::new("noop", 5))
    .await
    .unwrap()
    .entries
```

- [ ] Step 5: Add one deprecated compatibility test.

The test must use `#[allow(deprecated)]` locally and prove old calls still work through default wrappers.

### Task 4: Migrate `macaca-kernel::MemoryServiceAdapter`

**Files:**

- Modify: `macaca/crates/macaca-kernel/src/services.rs`
- Modify if needed: `macaca/crates/macaca-kernel/src/lib.rs`

- [ ] Step 1: Replace direct `MemoryStore` adapter semantics with facade semantics.

Preferred low-risk implementation:

- Keep type name `MemoryServiceAdapter` for API compatibility.
- Support a generic backend that can provide facade-first behavior.
- For `SessionMemory` test compatibility, either:
  - create a standard `MemoryManager` through `MemoryBackendFactory`, or
  - introduce a small local adapter path that maps `RememberText` to `MemoryEntry` for raw `MemoryStore`.

Recommended implementation if staying minimal:

```rust
pub struct MemoryServiceAdapter<M> {
    memory: M,
}
```

Then provide impls for facade-capable manager types:

```rust
#[async_trait]
impl<V, E> MemoryService for MemoryServiceAdapter<macaca_memory::MemoryManager<V, E>>
where
    V: macaca_memory::VectorStore + Send + Sync,
    E: macaca_memory::EmbeddingProvider + Send + Sync,
{
    async fn remember_text(&self, input: RememberText) -> MacacaResult<MemoryId> {
        self.memory.remember_text(input).await
    }

    async fn recall(&self, query: RecallQuery) -> MacacaResult<RecallResult> {
        self.memory.recall(query).await
    }
}
```

Add a parallel impl for `IsolatedMemoryManager<V, E>`.

- [ ] Step 2: Preserve legacy adapter behavior only behind deprecated calls.

If a raw `MemoryStore` adapter path must remain for compatibility, mark its constructor deprecated:

```rust
#[deprecated(note = "use MemoryServiceAdapter with MemoryManager or IsolatedMemoryManager facade backend")]
pub fn from_store(store: S) -> Self
```

Do not remove the old path in this change.

- [ ] Step 3: Update kernel tests to use `MemoryBackendFactory`.

Replace:

```rust
let session = macaca_memory::SessionMemory::new(Duration::from_secs(60));
let adapter = MemoryServiceAdapter::new(session);
```

with:

```rust
let dir = tempfile::tempdir().unwrap();
let factory = macaca_memory::MemoryBackendFactory::new(
    macaca_memory::MemoryBackendConfig::new(dir.path().to_path_buf())
        .session_ttl(Duration::from_secs(60)),
);
let adapter = MemoryServiceAdapter::new(factory.test_manager());
```

- [ ] Step 4: Update adapter tests to call `remember_text` / `recall`.

Expected assertion:

```rust
let id = adapter
    .remember_text(macaca_memory::RememberText::new("test memory"))
    .await
    .unwrap();
let results = adapter
    .recall(macaca_memory::RecallQuery::new("test", 10))
    .await
    .unwrap();
assert!(results.entries.iter().any(|entry| entry.id == id));
```

### Task 5: Grep guard and verification

**Files:**

- Possibly modify tests only.

- [ ] Step 1: Run formatting.

```bash
cd macaca
cargo fmt
```

- [ ] Step 2: Run focused tests.

```bash
cd macaca
cargo test -p macaca-agent services -- --nocapture
cargo test -p macaca-kernel services -- --nocapture
```

Expected: tests pass.

- [ ] Step 3: Run compile check.

```bash
cd macaca
cargo check -p macaca-memory -p macaca-agent -p macaca-kernel -p macaca-web
```

Expected: compile succeeds. Existing unrelated warnings are acceptable; new deprecated-call warnings in migrated upper production code are not acceptable.

- [ ] Step 4: Verify no upper production code calls deprecated memory service methods.

Run:

```bash
rg -n "memory_service\\(\\)\\.(store|retrieve)|MemoryServiceAdapter<|impl MemoryService for" \
  macaca/crates/macaca-agent/src macaca/crates/macaca-kernel/src \
  --glob '!*/tests/*'
```

Expected:

- No `memory_service().store/retrieve` production calls.
- Any `.store/.retrieve` hits must be unrelated persist/task/store traits or compatibility definitions.

- [ ] Step 5: Validate OpenSpec.

```bash
openspec validate migrate-memory-consumers-to-facade-primitives --strict
```

- [ ] Step 6: GitNexus detect changes before commit.

Run through MCP:

```text
gitnexus_detect_changes({ scope: "all", repo: "agent" })
```

Expected: affected scope is limited to `macaca-agent`, `macaca-kernel`, OpenSpec, and this plan.

## 7. Risks and mitigations

- **Risk: `macaca-agent -> macaca-memory` 新依赖扩大基础接口层依赖面。**
  - Mitigation: `macaca-memory` 是底层 crate，只依赖 `macaca-proto`，不会形成循环；这符合 `refactor-order.md` 中“先稳定底层 contract，再迁移主要消费方”的方向。

- **Risk: trait 方法变更影响外部自定义 `MemoryService` 实现。**
  - Mitigation: 新方法与旧方法保留兼容默认路径；workspace 内所有 impl 同步迁移；旧方法不删除。

- **Risk: raw `MemoryEntry` 兼容 wrapper 丢失 `created_at` / `expires_at`。**
  - Mitigation: deprecated compatibility wrapper 如果需要保持完全 fidelity，应构造 `MemoryEntry` 路径或引入 internal `remember_entry` helper；facade-first 新代码只承诺 text memory contract。

- **Risk: grep `.store/.retrieve` 误报大量其他 trait。**
  - Mitigation: 验证脚本聚焦 `memory_service().store/retrieve` 和 `MemoryServiceAdapter` 相关文件，人工确认剩余命中。

## 8. 执行顺序

1. 先创建并验证 OpenSpec。
2. 再迁移 `macaca-agent::MemoryService` trait 和 tests。
3. 再迁移 `macaca-kernel::MemoryServiceAdapter` 和 tests。
4. 最后跑格式化、focused tests、cargo check、grep guard、OpenSpec validate、GitNexus detect。
5. 通过后单独提交，commit message 建议：

```bash
git commit -m "refactor: migrate memory consumers to facade primitives"
```

## 9. 自检

- Scope 覆盖：覆盖 `macaca-agent` service contract、`macaca-kernel` adapter、OpenSpec、测试与 grep guard。
- Placeholder scan：无 TBD/TODO。
- 设计边界：不改变 framework working memory，不改 task/session/trace，不引入 app-specific 逻辑。
- 迁移策略：旧接口 deprecated 保留，新上层代码不再调用。
