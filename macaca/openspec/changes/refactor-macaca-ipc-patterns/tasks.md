## 1. OpenSpec

- [x] 1.1 为 `macaca-ipc` transport bridge / factory 重构补齐 proposal / design / tasks / delta spec
- [x] 1.2 运行 `openspec validate refactor-macaca-ipc-patterns --strict`

## 2. Transport Bridge

- [x] 2.1 新增统一 `IpcTransport` contract，提供 sender / receiver 抽象创建入口
- [x] 2.2 为 local transport 接入新 contract，保持现有行为不变
- [x] 2.3 为 nats transport 接入新 contract，保持现有行为不变

## 3. Factory And Compatibility

- [x] 3.1 新增 `IpcTransportKind` / `IpcTransportConfig`
- [x] 3.2 新增 `IpcTransportFactory`
- [x] 3.3 保留 `LocalBus` / `NatsBus` 兼容 facade，并委托到新原语
- [x] 3.4 将 `LocalBus::{sender,receiver}` / `NatsBus::{sender,receiver}` 标记为 deprecated，但不删除

## 4. Verification

- [x] 4.1 运行 `cargo test -p macaca-ipc -- --nocapture`
- [x] 4.2 运行 `cargo check -p macaca-ipc -p macaca-kernel`
- [x] 4.3 运行 workspace `cargo check`
- [x] 4.4 运行 GitNexus `detect_changes(scope: "all")`
- [x] 4.5 仅在真实完成后更新 checklist
