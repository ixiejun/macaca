## 1. OpenSpec

- [x] 1.1 创建 `migrate-ipc-consumers-to-transport-primitives` 的 proposal / design / tasks / delta spec
- [x] 1.2 运行 `openspec validate migrate-ipc-consumers-to-transport-primitives --strict`

## 2. Consumer Audit

- [x] 2.1 扫描上层 crate 中所有对 deprecated IPC 入口的真实调用点
- [x] 2.2 确认是否存在真实的 transport factory 消费需求；若无，不制造伪调用

## 3. Kernel Migration

- [x] 3.1 将 `macaca-kernel::IpcServiceAdapter` sender 边界收敛到 `macaca_ipc::DynMessageSender`
- [x] 3.2 将已发现的 deprecated IPC 调用点迁移到 `create_sender()` 或等价新入口
- [x] 3.3 保持 `IpcService` 行为与测试语义不变

## 4. Verification

- [x] 4.1 运行 `cargo test -p macaca-kernel -- --nocapture`
- [x] 4.2 运行 `cargo check -p macaca-ipc -p macaca-kernel`
- [x] 4.3 运行 workspace `cargo check`
- [x] 4.4 运行 GitNexus `detect_changes(scope: "all")`
- [x] 4.5 仅在真实完成后更新 checklist
