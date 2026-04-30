## 1. Preparation

- [x] 1.1 盘点所有上层 crate 中 `AgentExecutionEvent` 的高频 `match` 路径
- [x] 1.2 盘点所有上层 crate 中高频 proto config DTO 的手写构造路径
- [x] 1.3 盘点所有上层 crate 中用户可见的 proto 错误展示路径
- [x] 1.4 对首批高风险迁移符号运行 GitNexus impact，并记录 blast radius

## 2. Layer A: Core Event Consumers

- [x] 2.1 迁移 `macaca-web` 的高频 `AgentExecutionEvent` 消费到 visitor
- [x] 2.2 审计 `macaca-framework` 的 `AgentExecutionEvent` 消费路径，确认当前无需要迁移到 visitor 的高频重复消费点
- [x] 2.3 审计 `macaca-kernel` 的 `AgentExecutionEvent` 消费路径，确认当前主要为创建/转发，无需要迁移到 visitor 的高频重复消费点
- [x] 2.4 保持 SSE / trace / event payload 完全兼容
- [x] 2.5 运行相关 crate 测试与编译验证

## 3. Layer B: Builder / Adapter Entry Consumers

- [x] 3.1 审计 `macaca-app` 的 proto config 构造路径，确认当前无高频重复 `MacacaConfig` / `LlmProviderConfig` 手写构造点
- [x] 3.2 审计 `macaca-cli` 的 proto config 构造路径，确认当前无高频重复 `MacacaConfig` / `LlmProviderConfig` 手写构造点
- [x] 3.3 审计 `macaca-runtime-host` 的 proto config 构造路径，确认当前无高频重复 `MacacaConfig` / `LlmProviderConfig` 手写构造点
- [x] 3.4 迁移真实存在的用户可见 proto 错误展示到 `ProtoErrorAdapter`，并审计其余 crate 当前无直接 `MacacaError` 用户出口或不值得机械迁移
- [x] 3.5 运行相关 crate 测试与编译验证

## 4. Layer C: Remaining Consumer Cleanup

- [ ] 4.1 迁移 `macaca-task` / `macaca-tools` / `macaca-driver` / `macaca-skill` 的高频重复消费路径
- [ ] 4.2 迁移 `macaca-runtime` / `macaca-llm` / `macaca-memory` / `macaca-persist` / `macaca-ipc` / `macaca-gateway` / `macaca-sdk` 的高频重复消费路径
- [ ] 4.3 确认这些 crate 不再新增高频旧式消费路径

## 5. Contract Discipline

- [ ] 5.1 明确上层 crate 迁移后默认优先使用 visitor / builder / adapter
- [ ] 5.2 保持 `macaca-proto` 旧 API 仅作为兼容层，不作为上层默认新写法
- [ ] 5.3 不借迁移修改业务语义、wire schema、trace schema

## 6. Verification

- [x] 6.1 运行 `cargo check -p macaca-web -p macaca-framework -p macaca-kernel`
- [x] 6.2 运行 `cargo check -p macaca-app -p macaca-cli -p macaca-runtime-host`
- [x] 6.3 如有对应测试，运行分 crate 测试
- [x] 6.4 运行 workspace `cargo check`
- [x] 6.5 运行 `gitnexus_detect_changes(scope: "all")`
- [x] 6.6 更新本 checklist，仅在真实完成后标记
