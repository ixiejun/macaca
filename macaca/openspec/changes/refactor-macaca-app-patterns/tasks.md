## 1. Preparation

- [x] 1.1 运行 GitNexus impact：`AppRuntime` upstream，记录直接调用者和风险等级
- [x] 1.2 运行 GitNexus impact：`AppLoader` upstream，记录直接调用者和风险等级
- [x] 1.3 运行 GitNexus impact：`WorkflowEngine` upstream，记录直接调用者和风险等级
- [ ] 1.4 运行 GitNexus impact：`build_system_prompt` upstream，记录直接调用者和风险等级
- [x] 1.5 阅读 `macaca/crates/macaca-app/src/{loader,runtime,workflow,model}.rs`，确认当前 public API、默认 prompt、错误路径和测试覆盖

## 2. Behavior Lock Tests

- [x] 2.1 为 `AppRuntime::start_app_from_file` / `start_app` 增加当前行为锁定测试
- [x] 2.2 为 `AppRuntime` 成功/重复加载/停止/删除路径补齐兼容测试
- [x] 2.3 为 `WorkflowEngine::default_workflow_prompt` / `default_assistant_prompt` 增加 snapshot 或等价断言
- [x] 2.4 为 `WorkflowEngine::build_system_prompt` 增加 prompt 兼容测试
- [x] 2.5 为 `FULLSTACK-AUTODEV` 与 `NEWSROOM-AUTOWRITER` 准备 runtime/prompt fixture

## 3. AppRuntimeBuilder

- [x] 3.1 新增 `AppRuntimeBuilder`，承接 manifest + base_dir 装配逻辑
- [x] 3.2 将 validation / agent config resolve / loaded app assemble 显式拆分到 builder
- [x] 3.3 保留 `AppRuntime::start_app*` 入口并委托到 builder
- [x] 3.4 确认成功路径与错误路径行为不变

## 4. WorkflowPromptParts + Strategy

- [x] 4.1 抽出 `WorkflowPromptParts`，将 workflow prompt 拆成稳定片段
- [x] 4.2 引入 `WorkflowPromptStrategy` trait 和默认实现
- [x] 4.3 让 `WorkflowEngine::build_system_prompt` 走 template + strategy 渲染
- [x] 4.4 保持默认输出与当前 prompt 等价

## 5. Driver / Tool Selection Decoupling

- [x] 5.1 识别默认 prompt 中对具体 driver/tool 的硬编码依赖
- [x] 5.2 将 driver/tool 选择提示迁移到 capability/provider 输入层
- [x] 5.3 默认 strategy 不再写死单个 driver 名称
- [x] 5.4 验证 prompt 行为仍与现有应用兼容

## 6. Application Runtime Factory + Capability Composite

- [x] 6.1 新增默认 `ApplicationRuntimeFactory`，统一应用装配入口
- [x] 6.2 引入 application-level capability composite 内部结构
- [x] 6.3 保持 legacy capability 输出兼容
- [x] 6.4 确认 application-level 可见能力没有减少

## 7. Verification

- [x] 7.1 运行 `cargo fmt`
- [x] 7.2 运行 `cargo test -p macaca-app`
- [x] 7.3 运行 `cargo check -p macaca-app`
- [x] 7.4 如 public API 影响调用侧，运行 workspace `cargo check`
- [ ] 7.5 运行 `gitnexus_detect_changes(scope: "all")`，确认影响范围符合预期
- [x] 7.6 更新本 checklist，确保每项真实完成后再标记
