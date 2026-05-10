# Route C Workspace Topology Refactor 执行计划

## 目标

重构 Macaca Rust workspace 的目录组织，使文件系统能够体现 Route C 的“微内核 + 非内核能力服务化/模块化”架构。该变更应把 crate 移动到按 layer 分组的目录中，同时保持 package name、Rust crate name、public API、运行行为和现有 service contract 不变。

本计划遵循 `docs/openharmony-microkernel-architecture-for-macaca-agent-os.md`、`macaca/docs/agent-os-microkernel-boundaries.md`、`macaca/docs/route-c-architecture-governance.md`、`macaca/docs/route-c-serviceization-allowlist.md` 和 `macaca/docs/design_patterns.md`。

## 强约束

- 实施前必须先创建 OpenSpec。
- 本次 topology refactor 不重命名 Rust package 或 crate name。
- 不改变 public API、service ID、command name、route path、CLI command、wire format、session storage semantics、trace semantics 或 provider behavior。
- 第一轮不创建新 crate。
- 不删除 deprecated compatibility anchor。
- 变更必须机械、可审查、可回滚。
- 优先使用 `cargo metadata` 发现路径，避免继续硬编码 `crates/macaca-*`。
- 更新可执行门禁，让架构 topology 可被测试验证，而不是只停留在文档中。
- 按项目规则，在编辑和提交前运行 GitNexus impact/detect。
- 目录移动完成后，如果 GitNexus 报告索引过期，或验证完成后需要刷新索引，则运行 `npx gitnexus analyze`。

## 设计模式选择

- Layers：目录分组映射架构层级。
- Facade：`facade/macaca-sdk` 继续作为 shell-facing 入口。
- Bridge：`foundation/macaca-ipc` 和 `runtime/macaca-runtime-host` 让 service bus/runtime-host 边界可见。
- Adapter：`shells/*`、`services/macaca-gateway`、`services/macaca-driver`、`services/macaca-skill` 保持 adapter/plugin/service extension surface 语义。
- Registry：crate topology table 映射 package name 到预期 layer path。
- Specification：integration test 使用 `cargo metadata` 验证 package path。
- Memento：文档记录 old-to-new path mapping 和剩余迁移债务。

## 目标目录拓扑

```text
macaca/crates/
  README.md
  foundation/
    macaca-proto/
    macaca-ipc/
    macaca-persist/
  kernel/
    macaca-kernel/
  services/
    macaca-task/
    macaca-llm/
    macaca-memory/
    macaca-context/
    macaca-driver/
    macaca-skill/
    macaca-gateway/
    macaca-tools/
  runtime/
    macaca-runtime/
    macaca-runtime-host/
    macaca-framework/
  application/
    macaca-agent/
    macaca-app/
  facade/
    macaca-sdk/
  shells/
    macaca-web/
    macaca-cli/
  tests/
    macaca-integration-tests/
```

## Layer Map

| Crate | 新路径 | Route C 角色 |
| --- | --- | --- |
| `macaca-proto` | `crates/foundation/macaca-proto` | Protocol / ABI / service DTO / trace types |
| `macaca-ipc` | `crates/foundation/macaca-ipc` | Service bus / transport bridge |
| `macaca-persist` | `crates/foundation/macaca-persist` | Persistence contract 和当前 storage adapter |
| `macaca-kernel` | `crates/kernel/macaca-kernel` | Microkernel invariants |
| `macaca-task` | `crates/services/macaca-task` | Task service domain |
| `macaca-llm` | `crates/services/macaca-llm` | LLM service domain |
| `macaca-memory` | `crates/services/macaca-memory` | Memory service / memory fabric |
| `macaca-context` | `crates/services/macaca-context` | Context service / context engine |
| `macaca-driver` | `crates/services/macaca-driver` | Driver service/plugin adapter domain |
| `macaca-skill` | `crates/services/macaca-skill` | Skill service/package runtime domain |
| `macaca-gateway` | `crates/services/macaca-gateway` | Gateway service/plugin adapter domain |
| `macaca-tools` | `crates/services/macaca-tools` | Tool service compatibility/domain primitives |
| `macaca-runtime` | `crates/runtime/macaca-runtime` | Agentic runtime primitives |
| `macaca-runtime-host` | `crates/runtime/macaca-runtime-host` | Host-owned service runtime 和 provider wrapper |
| `macaca-framework` | `crates/runtime/macaca-framework` | Traced agent/framework execution seam |
| `macaca-agent` | `crates/application/macaca-agent` | Agent primitives 和 agent-facing contracts |
| `macaca-app` | `crates/application/macaca-app` | Application Framework / package / lifecycle |
| `macaca-sdk` | `crates/facade/macaca-sdk` | SystemFacade 和 developer-facing API |
| `macaca-web` | `crates/shells/macaca-web` | Web shell / GenUI / trace viewer |
| `macaca-cli` | `crates/shells/macaca-cli` | CLI shell / terminal adapter |
| `macaca-integration-tests` | `crates/tests/macaca-integration-tests` | Cross-layer governance 和 regression |

## 实施切片

### Slice 1：OpenSpec 提案

1. 创建 `openspec/changes/refactor-route-c-workspace-topology/`。
2. 添加 `proposal.md`、`design.md`、`tasks.md`。
3. 添加 delta spec，建议为 `workspace-topology/spec.md`，要求：
   - Route C layer directories 位于 `macaca/crates/` 下。
   - package name 和 crate name 保持稳定。
   - 通过 `cargo metadata` 验证 topology。
   - 不改变行为和 API。
   - 文档化 old-to-new mapping。
4. 验证：

```bash
openspec validate refactor-route-c-workspace-topology --strict
```

### Slice 2：先增加 Topology 文档和 Guard

1. 新增 `macaca/crates/README.md`，解释 Route C crate layers 和 old-to-new map。
2. 新增或扩展 integration-test support，加入 topology map：
   - 建议新增测试模块 `macaca-integration-tests/tests/route_c_workspace_topology`。
   - 使用 `cargo metadata --no-deps --format-version 1`。
   - 验证每个 package manifest path 以预期 layer path 结尾。
3. topology guard 可以与目录移动同一提交落地；若提前添加，则必须在移动前后都有明确预期。
4. 验证 Route C gate 仍可通过。

### Slice 3：机械更新 Cargo Path

1. 更新 `macaca/Cargo.toml`：
   - `members` 改为 `crates/<layer>/<crate>`。
   - `[workspace.dependencies]` 中内部 crate 的 `path` 改为新路径。
2. 除非单个 crate manifest 内存在未通过 workspace dependency 继承的相对 path dependency，否则不修改各 crate `Cargo.toml`。
3. 运行：

```bash
cargo metadata --no-deps --format-version 1
```

### Slice 4：移动 Crate 目录

只移动目录；该切片不修改 Rust 源码内部逻辑：

```text
crates/macaca-proto -> crates/foundation/macaca-proto
crates/macaca-ipc -> crates/foundation/macaca-ipc
crates/macaca-persist -> crates/foundation/macaca-persist
crates/macaca-kernel -> crates/kernel/macaca-kernel
crates/macaca-task -> crates/services/macaca-task
crates/macaca-llm -> crates/services/macaca-llm
crates/macaca-memory -> crates/services/macaca-memory
crates/macaca-context -> crates/services/macaca-context
crates/macaca-driver -> crates/services/macaca-driver
crates/macaca-skill -> crates/services/macaca-skill
crates/macaca-gateway -> crates/services/macaca-gateway
crates/macaca-tools -> crates/services/macaca-tools
crates/macaca-runtime -> crates/runtime/macaca-runtime
crates/macaca-runtime-host -> crates/runtime/macaca-runtime-host
crates/macaca-framework -> crates/runtime/macaca-framework
crates/macaca-agent -> crates/application/macaca-agent
crates/macaca-app -> crates/application/macaca-app
crates/macaca-sdk -> crates/facade/macaca-sdk
crates/macaca-web -> crates/shells/macaca-web
crates/macaca-cli -> crates/shells/macaca-cli
crates/macaca-integration-tests -> crates/tests/macaca-integration-tests
```

使用 `git mv` 或等价目录移动方式，让历史记录保持可理解。

### Slice 5：更新路径敏感测试和脚本

审计并更新移动后仍需要运行的路径敏感引用：

- `macaca/crates/tests/macaca-integration-tests/tests/route_c_baseline.rs`
- `macaca/crates/tests/macaca-integration-tests/tests/route_c_dependency_boundaries/gate.rs`
- `macaca/crates/tests/macaca-integration-tests/tests/task_api_migration_audit.rs`
- `scripts/check-cli-consumer-migration.sh`
- `scripts/check-web-cli-thin-shell.sh`
- `macaca/scripts/` 或顶层 `scripts/` 下其他活跃脚本。

能避免硬编码时，优先改为：

- 通过 `cargo metadata` 查找 package path。
- 使用 workspace-root 相对 glob，例如 `macaca/crates/**/macaca-web/src`。
- 如果文件本身是 topology guard，则使用显式新 layer path。

不要批量重写历史 research/proposal 文档中的旧路径，除非该路径是当前仍会执行的命令说明。新增 topology 文档应作为当前事实来源。

### Slice 6：治理文档更新

更新当前架构文档：

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/route-c-serviceization-allowlist.md`，仅在文本引用 flat path 或需要补充 topology 说明时修改。

文档必须说明：

- 文件系统 layer 不是依赖许可。
- dependency gate 仍然是 forbidden edge 的权威门禁。
- layer 移动不等于 API 迁移完成。
- 未来新增 crate 必须放入 Route C layer，并加入 topology guard。

### Slice 7：验证

运行：

```bash
openspec validate refactor-route-c-workspace-topology --strict
cargo metadata --no-deps --format-version 1
cargo fmt --all --check
cargo check --workspace
cargo test -p macaca-integration-tests route_c_dependency_boundaries
cargo test -p macaca-integration-tests --test route_c_baseline
cargo test -p macaca-integration-tests route_c_workspace_topology
```

如果目录移动暴露 package 层路径问题，补跑目标 package 测试：

```bash
cargo test -p macaca-sdk
cargo test -p macaca-runtime-host
cargo test -p macaca-web
cargo test -p macaca-cli
```

如果脚本有更新，运行受影响脚本：

```bash
scripts/check-cli-consumer-migration.sh
scripts/check-web-cli-thin-shell.sh
```

运行 GitNexus：

```bash
npx gitnexus detect-changes -r agent
npx gitnexus analyze
```

如果 GitNexus 在 impact/detect 前报告索引过期，先按项目规则运行 `npx gitnexus analyze`。

## 回滚策略

- 因为 package name 和源码 API 保持不变，回滚主要是路径级 revert。
- OpenSpec 获批后，目录移动应尽量放在单独提交中，便于回滚。
- 不把行为改动混入目录移动。
- 如果 Cargo metadata 在移动后失败，必须同时恢复 `macaca/Cargo.toml` 路径和目录布局。

## 预期 Diff

大规模变化：

- 所有 Rust workspace crate 的目录移动。
- `macaca/Cargo.toml` member 和 dependency path 更新。
- 活跃脚本/测试路径更新。

小规模变化：

- Rust 源码逻辑应几乎不变。
- 不改变 public API。
- 不改变 service behavior。

## 验证成功标准

- `cargo metadata` 能从新 layer path 列出全部 21 个 workspace package。
- `cargo check --workspace` 通过。
- Route C dependency boundary gate 通过。
- 新 workspace topology guard 通过。
- OpenSpec strict validation 通过。
- GitNexus detect 只报告预期的路径重组和少量 guard/doc 变更，不出现意外行为层 blast radius。

## 非目标

- 本次不创建 `macaca-store`、`macaca-payment`、`macaca-web3`、`macaca-evm` 或 `macaca-ui`。
- 不因为目录移动删除 allowlist rows。
- 不批量归档旧 OpenSpec changes，也不批量重写历史 proposal 中的旧路径。
- 不在 topology-only 提案里拆分 `macaca-runtime-host` 内部实现。
- 不做 Web/CLI 或 provider 逻辑迁移，除非该迁移已经在之前阶段完成。

## OpenSpec Change Id

建议 change id：

```text
refactor-route-c-workspace-topology
```
