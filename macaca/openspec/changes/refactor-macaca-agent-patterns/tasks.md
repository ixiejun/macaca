## 1. Preparation

- [x] 1.1 运行 GitNexus impact：`AgentServices` upstream，记录直接调用者和风险等级
- [x] 1.2 运行 GitNexus impact：`BasicAgent` upstream，记录直接调用者和风险等级
- [x] 1.3 运行 GitNexus impact：`AgentStateMachine` upstream，记录直接调用者和风险等级
- [x] 1.4 阅读 `macaca/crates/macaca-agent/src/**`，确认当前 public API、测试覆盖和调用点

## 2. Behavior Lock Tests

- [x] 2.1 为 `AgentStateMachine` 添加当前状态转移黄金测试
- [x] 2.2 为 `AgentServices` 缺省/空服务行为添加测试，确认无副作用
- [x] 2.3 为 `BasicAgent` 当前构造行为添加 snapshot 或等价断言
- [x] 2.4 为现有 capability 输出添加兼容性测试

## 3. AgentServices Facade + Null Object

- [x] 3.1 增加 `AgentServices` 只读 facade 方法，内部仍兼容现有字段
- [x] 3.2 增加 no-op 服务实现，例如 event sink / memory service 的空实现
- [x] 3.3 将 crate 内部安全调用点迁移到 facade 方法
- [x] 3.4 确认无服务场景不会产生额外 event、trace、memory 写入

## 4. BasicAgentBuilder

- [x] 4.1 新增 `BasicAgentBuilder`，提供与旧构造等价的默认值
- [x] 4.2 将 `BasicAgent::new` 保留并委托到 builder
- [x] 4.3 仅迁移 `macaca-agent` crate 内部构造点，不强制修改其他 crate
- [x] 4.4 添加 builder 行为测试，确认输出与旧构造一致

## 5. AgentLifecyclePolicy

- [x] 5.1 新增 `AgentLifecyclePolicy` trait 和默认实现
- [x] 5.2 将 `AgentStateMachine` 内部状态转移判断委托给默认 policy
- [x] 5.3 保留 `AgentStateMachine` 对外 API 不变
- [x] 5.4 运行状态转移黄金测试，确认新旧结果一致

## 6. Capability Composite

- [x] 6.1 新增 `AgentCapabilitySet` / capability node 结构
- [x] 6.2 支持 flatten 到旧 `Vec<AgentCapability>` 输出
- [x] 6.3 `BasicAgentBuilder` 支持从 legacy capability 输入构建 capability set
- [x] 6.4 确认现有 capability 对外展示和序列化不变

## 7. Verification

- [x] 7.1 运行 `cargo fmt`
- [x] 7.2 运行 `cargo test -p macaca-agent`
- [x] 7.3 运行 `cargo check -p macaca-agent`
- [x] 7.4 如 public API 变动影响调用侧，运行 `cargo check`
- [x] 7.5 运行 `gitnexus_detect_changes(scope: "all")`，确认影响范围符合预期
- [x] 7.6 更新本 tasks checklist，确保每一项真实完成后再标记
