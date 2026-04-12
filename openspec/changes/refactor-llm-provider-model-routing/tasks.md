## 1. Registry and Resolution

- [ ] 1.1 梳理并统一 `macaca-llm` 中现有 provider 注册入口与 `LlmRouter` 能力
- [ ] 1.2 定义统一的 model selection / route plan 结构，覆盖默认值、agent override、显式 provider:model 引用与 fallback
- [ ] 1.3 实现并测试 model resolver 的 precedence 规则与兼容解析

## 2. Framework Integration

- [ ] 2.1 在 `macaca-framework` 中新增基于 router 的 `ChatModel` adapter
- [ ] 2.2 让 framework runner 的 coordinator / planner / worker 构建统一走 routed model
- [ ] 2.3 移除 framework runner 中对“单 provider + 单模型字符串”的隐式假设

## 3. Web Bootstrap Refactor

- [ ] 3.1 将 `macaca-web` 启动层改为从 config 初始化 provider registry / router
- [ ] 3.2 删除 `macaca-web/src/lib.rs` 中硬编码 provider 构造分支
- [ ] 3.3 让 `AppState` 持有统一的 router / routed model factory，而不是单 provider 假设

## 4. Compatibility and Verification

- [ ] 4.1 保持现有 `default_provider` 与 `providers.<name>.default_model` 配置兼容
- [ ] 4.2 补充单元测试：旧配置、agent model override、provider 显式选择、fallback chain
- [ ] 4.3 手动验证 fullstack-autodev 的 coordinator / planner / worker 都能正常命中目标模型
- [ ] 4.4 `cargo check` 与相关测试通过
