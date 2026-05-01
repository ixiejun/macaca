## 1. Spec

- [x] 1.1 创建 `migrate-task-consumers-to-canonical-apis` proposal / design / tasks / delta spec
- [x] 1.2 审计上层真实调用面，确认 deprecated task API 已迁移完成

## 2. Guard

- [x] 2.1 增加针对上层 task consumer 的源码级 audit test
- [x] 2.2 覆盖 `macaca-tools`、`macaca-web`、`macaca-integration-tests` 的关键消费文件
- [x] 2.3 禁止重新引入旧 task API 调用模式，并在失败信息中指出 canonical replacement

## 3. Verification

- [x] 3.1 运行 `openspec validate migrate-task-consumers-to-canonical-apis --strict`
- [x] 3.2 运行 `cargo test -p macaca-integration-tests --test task_api_migration_audit -- --nocapture`
- [x] 3.3 运行 `cargo check -p macaca-integration-tests`
- [x] 3.4 运行 `gitnexus_detect_changes(scope: "all")`
- [x] 3.5 更新 tasks.md 使其与真实状态一致
