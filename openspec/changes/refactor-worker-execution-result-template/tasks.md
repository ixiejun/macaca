## 1. Implementation

- [x] 1.1 在 `loop_manager.rs` 中新增 worker 执行模式枚举或参数，用于区分 normal 与 retry 的差异
- [x] 1.2 在 `loop_manager.rs` 中新增 helper，统一处理 worker agent reply 成功分支
- [x] 1.3 在 `loop_manager.rs` 中新增 helper，统一处理 worker agent reply error/panic/timeout 分支
- [x] 1.4 将 `WorkerEvent::TaskClaimed` 分支切换为 helper，保持 `WORKER_TASK_SUCCESS` 与 `WORKER_SUBMIT_REVIEW` 行为不变
- [x] 1.5 将 `WorkerEvent::RetryTask` 分支切换为 helper，保持 retry 错误文案与 `retry_success` trace detail 不变

## 2. Tests

- [x] 2.1 添加或更新 unit test，覆盖 normal success 空输出 fallback summary
- [x] 2.2 添加或更新 unit test，覆盖 retry success 空输出 fallback summary
- [x] 2.3 添加或更新 unit test，覆盖 normal/retry panic 与 timeout 错误文案保持不同

## 3. Verification

- [x] 3.1 运行 `openspec validate refactor-worker-execution-result-template --strict`
- [x] 3.2 运行 worker result helper 相关单元测试
- [x] 3.3 运行 `cargo check -p macaca-web`
- [x] 3.4 运行 GitNexus `detect_changes(scope=staged)` 并确认影响范围符合本次局部重构
