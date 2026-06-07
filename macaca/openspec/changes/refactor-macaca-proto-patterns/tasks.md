## 1. Preparation

- [x] 1.1 运行 GitNexus impact：`AgentExecutionEvent` upstream，记录直接调用者和风险等级
- [x] 1.2 运行 GitNexus impact：首批目标 config DTO upstream，记录直接调用者和风险等级
- [x] 1.3 运行 GitNexus impact：目标 proto error 类型 upstream，记录直接调用者和风险等级
- [x] 1.4 阅读 `macaca/crates/macaca-proto/src/**`，确认当前 public DTO、serde 约束、默认值和测试覆盖

## 2. Behavior Lock Tests

- [x] 2.1 为核心 event enum 添加序列化兼容测试，锁定当前 JSON / serde 行为
- [x] 2.2 为首批 config DTO 添加默认值和手写构造等价测试
- [x] 2.3 为目标 proto error 添加 display/code 兼容测试
- [x] 2.4 明确并测试 visitor / builder 引入后不改变现有 wire schema

## 3. Event Visitor

- [x] 3.1 为核心 event enum 新增 visitor trait 和 `accept()` 入口
- [x] 3.2 保持旧 enum 和旧 `match` 调用方式不变
- [x] 3.3 添加 visitor 行为测试，确认各 variant 分发正确
- [x] 3.4 确认新增 visitor 不影响 serde 和现有 payload

## 4. Config Builder

- [x] 4.1 选定第一批高频 config DTO，并新增 builder
- [x] 4.2 builder 默认值必须与现有 `Default` / 常用构造等价
- [x] 4.3 仅迁移 `macaca-proto` crate 内部测试或新增测试使用 builder
- [x] 4.4 保留旧 struct 初始化路径，不要求调用侧立即迁移

## 5. Proto Error Adapter

- [x] 5.1 为目标 proto error 增加统一 display/code 适配入口
- [x] 5.2 保持旧错误类型与原始语义不变
- [x] 5.3 添加适配行为测试，确认用户可见信息一致
- [x] 5.4 不在 proto 中引入 HTTP / retry / recovery 策略

## 6. Contract Boundary

- [x] 6.1 在 spec 中明确 DTO 只承载 contract，不承载 runtime strategy
- [x] 6.2 明确 task/framework/kernel 的策略职责仍在上层
- [x] 6.3 确认本轮没有把新的业务策略写回 `macaca-proto`

## 7. Verification

- [x] 7.1 运行 `cargo fmt -p macaca-proto`
- [x] 7.2 运行 `cargo test -p macaca-proto`
- [x] 7.3 运行 `cargo check -p macaca-proto`
- [x] 7.4 如 public API 影响调用侧，运行 workspace `cargo check`
- [x] 7.5 运行 `gitnexus_detect_changes(scope: "all")`，确认影响范围符合预期
- [x] 7.6 更新本 tasks checklist，确保每一项真实完成后再标记
