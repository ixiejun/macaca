# Change: Complete Memory Fabric Runtime

## Why

`docs/memory-system-openclaw-hermes-research.md` 定义了 Macaca 记忆系统的六个阶段。当前代码已经具备核心 scope/facade、builtin private/shared adapters、active recall 和部分 provider/vector/governance 能力，但审计确认 Phase 2、Phase 3、Phase 5、Phase 6 仍存在 stub、未接入生产路径或任务状态与代码现实不一致的问题。

本变更以增量方式补齐完整 memory fabric runtime：建立生产级 `MemoryRuntimeFacade`，把 provider runtime、embedding registry、vector backend、query pipeline、knowledge/wiki、governance autonomy 和 `macaca-web` 生产消费路径统一收敛到可插拔、可观测、scope-safe 的运行时边界。

## What Changes

- 引入生产 `MemoryRuntimeFacade`，组合 provider routing、active recall、knowledge compilation、governance 和 status。
- 完成 memory provider runtime，使 profile/slot 能解析真实 provider 实例，并保证 MCP provider capability 状态真实。
- 增加 embedding provider registry，并以 Decorator 提供 cache、timeout、retry、metrics。
- 增加 vector backend conformance harness，并完成 builtin/Milvus 拓扑覆盖和替代 backend 可复用契约。
- 增加 query pipeline strategy stack，支持 keyword、vector、hybrid、filtered 和 rerank-compatible 搜索；embedding/vector 不可用时 keyword fallback。
- 增强 knowledge/wiki runtime：确定性 contradiction detection、conflict groups、project decision log、wiki digest artifact、citation/evidence references。
- 增强 governance autonomy runtime：durable audit/candidate/tombstone snapshot、automatic candidate capture seam、compaction/dreaming truthful seam、provider migration checkpoint。
- 迁移 `macaca-web` 生产 memory consumers，使 active recall、explicit memory tools、knowledge digest 通过 runtime facade 或 facade-backed adapter 访问记忆。
- 修正已有 OpenSpec task 中与代码现实不一致的 false-complete 状态。

## Impact

- Affected specs: `macaca-memory-fabric-runtime`, `macaca-memory-provider-runtime`, `macaca-memory-vector-backend`, `macaca-memory-governance`, `active-vector-memory-context`, `knowledge-digest-context`
- Affected crates:
  - `macaca/crates/macaca-memory`
  - `macaca/crates/macaca-context`
  - `macaca/crates/macaca-web`
  - optional `macaca/crates/macaca-integration-tests`
- Compatibility:
  - 现有 legacy manager/store APIs 保留，不删除。
  - `TestMemoryManager` 可继续作为 builtin backing store 或测试 adapter，但不再是生产 canonical memory boundary。
  - 默认测试不得依赖外部 Milvus、LanceDB、Qdrant、MCP 或 remote providers。
  - 不引入 app-specific、agent-name-specific、workflow-specific 或 provider-vendor-specific 业务逻辑。
