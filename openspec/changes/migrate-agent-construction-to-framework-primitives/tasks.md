## 1. Framework Contract

- [ ] 1.1 在 `macaca-framework` 中定义 traced agent construction contract：request、intent、trace context、factory trait
- [ ] 1.2 让新 contract 直接消费 `AgentServices`、`AgentCapabilitySet`、lifecycle config，而不是重新发明平行结构
- [ ] 1.3 定义 toolkit contributor / trace sink 等扩展接口，避免把 web-specific 资源直接放进 framework API
- [ ] 1.4 为新 contract 写最小单测或 compile-time usage fixture

## 2. Web Adapter Layer

- [ ] 2.1 将 `macaca-web/src/framework_runner.rs` 中通用构建流程改为委托 framework primitive
- [ ] 2.2 将 `AppState`、workspace、session、EventLog/SSE、skill/MCP 工具注入整理为 web adapter / contributor
- [ ] 2.3 让 web adapter 显式组装 `AgentServices` 与 capability input，再交给 framework factory
- [ ] 2.4 保留现有 public builder 入口，作为兼容 facade，不改变调用侧签名

## 3. Coordinator Migration

- [ ] 3.1 将 coordinator 构建路径迁移到 framework primitive
- [ ] 3.2 coordinator 路径改为消费新的 builder-style request，而不是继续在 web 内部散装参数
- [ ] 3.3 保持 pause/resume middleware、SSE hook、EventLog 持久化行为不变
- [ ] 3.4 验证 `chat_v2` 主链路中 coordinator 行为与当前一致

## 4. Planner / Worker Migration

- [ ] 4.1 将 planner decomposition builder 迁移为 framework intent + request
- [ ] 4.2 将 planner review/follow-up builder 迁移为 framework intent + request
- [ ] 4.3 将 worker builder 迁移为 framework intent + request
- [ ] 4.4 这些路径都必须消费 `AgentServices` facade、capability set 和统一 lifecycle/build contract
- [ ] 4.5 保持 task lifecycle event、agent activity、trace 和 tool visibility 行为不变

## 5. Task-side Decoupling

- [ ] 5.1 定义 task 侧可依赖的 execution launcher contract，不让 task 逻辑依赖 web helper 命名
- [ ] 5.2 将相关 planner/worker 执行意图改为通过 contract 表达
- [ ] 5.3 让 task 侧通过 contract 获取“基于 macaca-agent 新抽象构建”的执行能力，而不是感知底层装配
- [ ] 5.4 保持 PlanLoop / WorkerLoop 调度语义、waker 行为和 todo 状态流转不变

## 6. Verification

- [ ] 6.1 运行 `cargo fmt`
- [ ] 6.2 运行 `cargo check -p macaca-agent -p macaca-framework -p macaca-web -p macaca-task`
- [ ] 6.3 运行覆盖 coordinator/planner/worker 的相关单测或集成测试
- [ ] 6.4 验证 live SSE trace、EventLog 持久化、浏览器刷新恢复行为不变
- [ ] 6.5 验证各 intent 下的 tool visibility 与当前一致
- [ ] 6.6 验证 `AgentServices` facade/no-op 和 capability flatten compatibility 在新构建路径上保持不变
- [ ] 6.7 运行 `gitnexus_detect_changes(scope: "all")` 确认影响范围符合预期

