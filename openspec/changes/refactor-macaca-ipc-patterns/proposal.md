# Change: Refactor macaca-ipc Around Transport Bridge Primitives

## Why

`macaca-ipc` 当前只有 `LocalBus` 和 `NatsBus` 两套薄实现，发送、订阅、transport 选择和 transport 特定细节仍然散落在具体类型里。  
这会让后续增加 transport、统一配置入口、接入可靠性横切能力时继续放大 if/else 和重复 glue code。

同时，当前旧入口 `LocalBus::{sender,receiver}` / `NatsBus::{sender,receiver}` 仍是主要使用路径。为了后续迁移可控，需要先提供新原语，再把旧入口明确标记为 deprecated 以便调用方逐步收敛，但暂时不删除。

## What Changes

- 为 `macaca-ipc` 引入统一的 transport bridge 原语和 factory 入口。
- 保留现有 `MessageSender` / `MessageReceiver` 语义不变。
- 保留 `LocalBus` / `NatsBus` 及其 `sender()` / `receiver()` 旧入口，但标记为 deprecated。
- 新增 transport kind / config / factory，使 transport 选择不再依赖调用侧手写分支。
- 暂不实现 codec adapter 和 resilient proxy，只为后续切片保留 additive-first 扩展位。

## Impact

- Affected specs: `macaca-ipc-core`
- Affected code:
  - `macaca/crates/macaca-ipc/src/lib.rs`
  - `macaca/crates/macaca-ipc/src/bus.rs`
  - `macaca/crates/macaca-ipc/src/local.rs`
  - `macaca/crates/macaca-ipc/src/nats.rs`
- Compatibility:
  - 现有上层调用保持可编译、可运行
  - 旧入口将出现 deprecated 标记，作为后续迁移查找点
