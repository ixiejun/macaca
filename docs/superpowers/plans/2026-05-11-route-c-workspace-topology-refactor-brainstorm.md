# Route C Workspace Topology Refactor 头脑风暴

## 问题

Macaca 已经根据 `docs/openharmony-microkernel-architecture-for-macaca-agent-os.md`、`macaca/docs/agent-os-microkernel-boundaries.md` 和 Route C S0-S12 治理规则，对大量非内核能力完成了服务化与模块化。代码职责已经逐步转向微内核、runtime-host 托管服务、SDK Facade、可选模块和 thin shell，但 workspace 目录仍然把所有 crate 平铺在 `macaca/crates/` 下。

这种平铺结构会隐藏真实架构：

- Kernel primitive、protocol contract、system service、optional module、application framework、SDK facade、shell adapter、integration tests 在目录上看起来是同一层级。
- 新贡献者无法通过文件结构判断哪些 crate 是可替换服务，哪些 crate 是 base OS 不变量。
- 依赖门禁按 crate 名称分类，但文件系统没有强化这些边界。
- 平铺结构更容易诱导后续新增宏内核式依赖，因为层级边界不直观。

本次期望主要是结构性调整：把 crate 目录移动到按架构层划分的分组中，同时保持 package name、Rust crate name、public API 和运行行为不变。

## 当前证据

- `macaca/Cargo.toml` 当前把所有 workspace member 写成 `crates/macaca-*`。
- `macaca/docs/agent-os-microkernel-boundaries.md` 已经定义了 protocol、microkernel、service bus、system service、application framework、plugin、optional module、SDK、presentation shell 等所有权层级。
- `macaca/docs/route-c-architecture-governance.md` 通过规则和可执行依赖门禁强化同一套逻辑所有权。
- `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries/allowlist.rs` 和相关 gate 代码主要按 crate 名称分类依赖规则。实现时可以保持 crate 名称稳定，只移动目录。
- 许多文档和脚本提到 `macaca/crates/macaca-*`，需要兼容或更新策略。
- Cargo 支持任意深度的 workspace member 路径，因此目录分组可以在不重命名 package 的情况下完成。

## 设计模式视角

这是架构目录重构，不是行为重构。适用的模式包括：

- Layers：文件系统层级应映射 Route C 所有权层级。
- Facade：`macaca-sdk` 和 presentation shell 应在视觉上位于 service/runtime 层之上。
- Bridge：service bus/runtime-host 边界应直观体现 shell-facing client 与 provider implementation 之间的桥接。
- Adapter：shell crate 和 plugin/gateway/driver adapter 应被归入 adapter/extension 语义，而不是 base OS。
- Registry：crate topology metadata 应显式化，并由 topology guard 检查。
- Specification：workspace topology 规则应成为可执行测试，而不仅是文档说明。
- Memento：迁移债务和旧路径映射应被记录，便于后续检索和回滚。

## 方案 A：保持平铺 `crates/`，只补文档

### 结构

保持所有 crate 在 `macaca/crates/macaca-*`，新增 `macaca/docs/route-c-workspace-topology.md` 映射每个 crate 的 Route C 层级。

### 优点

- 实施风险最低。
- 不需要修改 Cargo path。
- 不会造成脚本和文档路径震荡。

### 风险

- 没有解决核心问题：文件系统仍然表达平铺架构。
- 后续贡献者仍然必须先读文档才能理解边界。
- 依赖违规在目录层面仍然很容易发生。

### 结论

不采用。用户明确要求文件目录组织结构也体现服务化与模块化，仅补文档不够。

## 方案 B：在 `macaca/crates/` 下按架构层移动 crate

### 结构

使用 `macaca/crates/<layer>/<crate>/`，同时保留所有 package name：

```text
macaca/crates/
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

### 优点

- 以较小 Cargo 改动直接提升架构可读性。
- package name 和 Rust crate name 保持不变，最大限度降低代码改动。
- 所有 Rust workspace crate 仍然保留在 `macaca/crates` 下，可以兼容部分既有路径假设。
- 可以新增 topology guard，断言 crate manifest path 符合 Route C 层级。

### 风险

- 许多文档、脚本、测试提到 `macaca/crates/macaca-*`。
- 部分测试假设 integration tests 位于 `macaca/crates` 下；该假设仍成立，但需要检查精确相对路径。
- `cargo metadata` 输出的 manifest path 会变化，GitNexus 索引可能需要重建。
- Git diff 体积会很大，虽然主要是目录移动。

### 缓解措施

- 只移动目录，不改 package name 和 Rust crate name。
- 在同一个切片里更新 `macaca/Cargo.toml` workspace members 和 workspace dependency paths。
- 移动前或移动后立即加入 `route_c_workspace_topology` guard。
- 能用 `cargo metadata` 发现路径的脚本/测试，不再硬编码旧路径。
- topology 切片不修改 `src/` 内部逻辑。

### 结论

推荐采用。它能让 Route C 拓扑在文件系统中可见，同时保持行为和 public API 稳定。

## 方案 C：把 crate 移到顶层 layer 目录，完全移出 `crates/`

### 结构

```text
macaca/
  foundation/
  kernel/
  services/
  runtime/
  application/
  facade/
  shells/
  tests/
```

### 优点

- 架构信号最强。
- `crates/` 不再暗示所有 package 都是平级。

### 风险

- 对文档、脚本、测试和开发习惯的路径冲击最大。
- 现有 integration-test 通过 `macaca/crates` 定位 workspace root 的假设会失效。
- 对期望 Rust package 位于 `crates/` 下的外部工具不友好。

### 结论

第一阶段不采用。该方案更干净，但过于激进；Route C 当前更适合小步、可逆的结构迁移。

## 方案 D：保持原 crate 目录，新增 layer wrapper symlink

### 结构

保留 `macaca/crates/macaca-*`，增加类似 `macaca/layers/services/macaca-llm -> ../../crates/macaca-llm` 的符号链接分组。

### 优点

- 不需要修改 Cargo path。
- 能提供一定视觉分组。

### 风险

- 同一个 crate 会出现两个看似有效的源码路径。
- 符号链接对工具、编辑器、CI、压缩包和跨平台环境都更脆弱。
- 可能混淆 GitNexus、rust-analyzer 和脚本。

### 结论

不采用。架构信号弱，还会引入文件系统歧义。

## 方案 E：移动目录的同时拆分新 crate

### 结构

移动目录，并创建 `macaca-store`、`macaca-payment`、`macaca-web3`、`macaca-evm`、`macaca-ui` 等新 crate。

### 优点

- 长期拓扑可以更精确。
- Store/Payment/Web3/EVM 可以从 `macaca-runtime-host` 中逐步独立。

### 风险

- 这已经不再是单纯结构重构。
- 新 crate 需要依赖门禁、API surface、迁移 spec 和实现改动。
- 会把 topology refactor 和 ownership refactor 混在一起，显著提高回归风险。

### 结论

延期。第一轮 topology refactor 不新增 crate。未来可以在路径已体现当前所有权后，再用独立提案拆分 Store、Payment、Web3、EVM、UI 等能力。

## 推荐拓扑

采用方案 B，并使用保守层级名称：

```text
macaca/crates/
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

## 层级归属理由

| 层级 | Crates | 理由 |
| --- | --- | --- |
| `foundation` | `macaca-proto`, `macaca-ipc`, `macaca-persist` | 共享 contract、service bus/transport、persistence contract/adapter 基础。它们低于 kernel/services，但不是业务 provider。 |
| `kernel` | `macaca-kernel` | 微内核不变量、registry、scheduler、policy、trace/task/session primitive。 |
| `services` | `macaca-task`, `macaca-llm`, `macaca-memory`, `macaca-context`, `macaca-driver`, `macaca-skill`, `macaca-gateway`, `macaca-tools` | 可替换 system service domain，以及 provider-neutral service contract/adapter。 |
| `runtime` | `macaca-runtime`, `macaca-runtime-host`, `macaca-framework` | Host/service lifecycle、framework execution seam、agentic runtime 和 middleware。 |
| `application` | `macaca-agent`, `macaca-app` | Agent primitive 与 Application Framework/package/application lifecycle。 |
| `facade` | `macaca-sdk` | 稳定的 shell/application-facing system facade 和 developer API。 |
| `shells` | `macaca-web`, `macaca-cli` | Presentation shell、terminal/HTTP adapter。 |
| `tests` | `macaca-integration-tests` | 跨层级可执行治理和回归测试。 |

## 关键风险

- **路径硬编码风险：** 脚本和文档大量使用 `macaca/crates/macaca-*`，简单移动可能破坏迁移审计和 guard 脚本。
- **依赖门禁漂移：** 当前 gate 按 crate name 分类，移动后还应校验 layer path。
- **GitNexus 索引过期：** 移动 crate 目录会改变大量文件路径，需要执行 `npx gitnexus analyze`。
- **Cargo workspace path 震荡：** 所有 workspace dependency path 必须原子更新。
- **OpenSpec 路径噪音：** 活跃变更中有大量旧路径引用。批量更新历史提案会制造噪音，但新 topology 文档必须说明旧路径引用的处理原则。
- **编辑器/工具缓存：** rust-analyzer 和 target/flycheck 可能需要重新加载 workspace。

## 风险控制

- 不重命名 package 或 Rust crate。
- topology move 期间不改变 Rust module path 或 public API。
- 使用 `git mv` 语义移动目录。
- 目录移动与 `macaca/Cargo.toml` 路径更新放在同一个实现切片。
- 新增 topology guard，用 `cargo metadata` 检查 workspace package manifest path 是否符合 Route C layer map。
- 对路径敏感的测试和脚本优先改为从 `cargo metadata` 发现 package path。
- 添加 `macaca/crates/README.md`，记录层级模型和 old-to-new mapping。
- 移动后运行 `cargo metadata`、`cargo fmt --all --check`、`cargo check --workspace`、Route C dependency boundaries、Route C baseline 和 GitNexus detect/analyze。

## 待确认问题

- `macaca-framework` 放在 `runtime/` 还是 `application/`。当前 Route C 文本将它视为 traced agent/middleware/MCP primitive，且经常参与 runtime execution，因此第一阶段建议放在 `runtime/`。
- `macaca-persist` 放在 `foundation/` 还是 `services/`。治理文档把 persistence 视为 service contract，但它足够基础，先放在 `foundation/` 更清晰。未来真正 Persistence Service 实现时可再拆分或重分类。
- `macaca-tools` 放在 `services/` 还是 `runtime/`。由于 tools 是可替换 capability surface，且 allowlist 中仍作为 Tool/Skill Service 债务存在，放在 `services/` 更符合目标。
- 是否现在为 Web3/EVM optional module 建独立目录。当前具体实现仍在 `macaca-runtime-host` 中，纯目录重构不应创建空 optional crate。

## 建议

推进一个 OpenSpec change：`refactor-route-c-workspace-topology`，随后按小步骤实施：

1. 定义目标 topology 和 topology guard。
2. 更新 Cargo workspace paths。
3. 把 crate 目录移动到 Route C layer group。
4. 更新路径敏感脚本、测试和当前治理文档。
5. 验证 workspace 行为和依赖门禁。
6. 移动完成后重建 GitNexus 索引。

第一轮实施应只做 topology refactor。Store、Payment、Web3、EVM、UI、Persistence Service 等新 crate 拆分，应保留为未来独立提案。
