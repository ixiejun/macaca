# macaca-sdk 设计模式渐进式重构 Brainstorm 设计记录

## 背景

`macaca-sdk` 位于重构顺序表的阶段 4：应用语义与系统协调层。它依赖 `macaca-agent`、`macaca-kernel`、`macaca-llm`、`macaca-proto`、`macaca-tools`，被 `macaca-app`、`macaca-web`、integration tests 和应用开发者间接消费。

当前 `macaca-sdk` 文件很少：

- `src/config.rs`：`AgentConfig`、`AgentSkillsConfig`、`CapabilityDef`，负责 YAML/TOML 解析和基础字段校验。
- `src/builder.rs`：`AgentBuilder` 和 `DeclarativeAgent`，当前 builder 直接产出运行时 agent。
- `src/persona.rs`：`AgentPersona`，按固定 Markdown 文件顺序加载 persona 并拼系统 prompt。
- `src/registry_api.rs`：`register_from_config` / `register_from_file`，直接接收 `macaca_kernel::Kernel` 并调用 `kernel.register_agent`。
- `src/lib.rs`：导出 SDK 公共 API。

当前职责边界已经能工作，但还不是理想的 Agent OS SDK 边界：

- `AgentBuilder` 同时承担 config validation、manifest field mapping、runtime `DeclarativeAgent` 构造。它是 Builder，但还没有产出 framework/app 可复用的中间 spec。
- `DeclarativeAgent` 实现 `macaca_agent::Agent`，因此 SDK builder 和具体 runtime agent 绑定较紧。
- `AgentPersona` 只有加载和 prompt 拼接，没有 prototype/override 能力，多个 agent 复用 persona 时需要复制字段。
- `AgentConfig::validate` 是单体校验方法，allowed tools、skills、driver、MCP 等未来策略无法插拔。
- `registry_api` 直接依赖 `Kernel`，对外开发者看起来方便，但 SDK facade 还没有把注册、校验、trace policy、kernel adapter 统一收口。

GitNexus 观察：

- `AgentBuilder`、`DeclarativeAgent`、`AgentPersona` 当前图谱上游影响为 LOW，适合先做 additive primitives。
- `register_from_config` 上游影响为 CRITICAL，直接影响 `macaca-app::start_app`、web `start_server`、integration tests。它必须后置，只做兼容 facade/adapter，不直接改注册语义。

## 设计模式适配

本轮严格按 `macaca/docs/design_patterns.md` 和 `macaca/docs/design-pattern-refactor-plans/macaca-sdk.md` 里的模式做小步重构：

- **Builder + Abstract Factory**：新增 `AgentSpec` 作为 SDK builder 的中间产品。`AgentBuilder` 先能产出 spec，再由 factory/adapter 产出 `DeclarativeAgent`。
- **Prototype**：新增 persona prototype 和 overrides，支持从同一个 persona 原型派生多个 agent persona，不修改原型本身。
- **Chain of Responsibility**：新增 `SdkValidationChain`，把 manifest 基础校验、tool policy 校验、skill policy 校验、driver/MCP 可见性校验拆成可组合 validator。
- **Facade / Adapter**：新增 `MacacaSdk` facade 和 kernel registry adapter，让上层不直接理解 `Kernel` 注册细节，同时保留旧函数作为兼容入口。
- **Decorator / Policy**：SDK 注册路径必须携带 trace policy 元数据，保证后续不会从 SDK 生成 untraced agent。

## 可选方案

### 方案 A：只抽 `AgentSpec`

做法：

- 新增 `AgentSpec`。
- `AgentBuilder::build_spec()` 返回 spec。
- `build()` / `build_with_manifest()` 内部委托 spec 转换。

优点：

- 风险最低。
- 不触碰 CRITICAL 的 `registry_api` 注册链路。
- 为后续 framework traced factory 打基础。

缺点：

- persona prototype、validation chain、MacacaSdk facade 仍缺失。
- 不能完整覆盖 `macaca-sdk.md` 的渐进式重构计划。

结论：适合作为第一切片，但不是完整提案。

### 方案 B：一个提案覆盖四个小切片，按 additive-first 分步实施

做法：

- 第一切片新增 `AgentSpec`，旧 builder API 委托新 spec。
- 第二切片新增 persona prototype/overrides。
- 第三切片新增 validation chain，`AgentConfig::validate` 可保留但委托 chain。
- 第四切片新增 `MacacaSdk` facade、registry adapter 和 trace policy 元数据，旧 `register_from_config` 保留并委托 facade。

优点：

- 覆盖 `macaca-sdk.md` 的完整计划。
- 每个切片都可独立测试、独立回滚。
- 先做 LOW 影响面的 builder/persona/validation，再处理 CRITICAL 注册链路。
- 旧 API 可 deprecated 但不删除，方便后续消费方迁移。

缺点：

- 提案范围比单切片大，需要 OpenSpec 里明确每个切片的边界。
- 如果一次实现全部切片，必须严格按顺序验证，避免 registry facade 改坏 app/web 启动。

结论：推荐。

### 方案 C：直接迁移到 macaca-framework traced agent factory

做法：

- `AgentBuilder` 不再产出 `DeclarativeAgent`，直接产出 framework traced build request。
- `register_from_config` 直接走 framework traced construction。

优点：

- 最终形态更接近“所有 agent 都走 trace 路径”。
- 可以一次性去掉 untraced declarative agent 的主要来源。

缺点：

- 风险过高，会同时触碰 SDK、app runtime、framework runner、kernel registration。
- 当前 `register_from_config` 已经是 CRITICAL，直接改运行时注册语义容易影响 web 启动和应用加载。
- 不符合“小步、1:1、可回滚”的工作约定。

结论：不推荐作为本轮目标。应在 `AgentSpec` 和 trace policy 元数据稳定后，另开消费方迁移提案。

## 推荐方案

采用方案 B：一个 OpenSpec 提案覆盖四个小切片，但实施时严格按顺序推进。

切片顺序：

1. `AgentSpec` 中间产品：让 `AgentBuilder` 先产出稳定声明模型，旧 `build()` 行为 1:1 保持。
2. Persona Prototype：支持 `PersonaPrototype::instantiate(overrides)`，不改变现有 `AgentPersona::load_from_directory` 行为。
3. SDK Validation Chain：把现有 `AgentConfig::validate` 拆成 validator chain，默认 chain 保持现有校验语义。
4. SDK Facade + Trace Policy：新增 `MacacaSdk`、registry adapter 和 trace policy 元数据，旧 `register_from_config` 委托新 facade 并标记 deprecated。

## 风险与控制

- 风险：`register_from_config` 影响 app/web 启动注册链路。
  控制：前三个切片不触碰注册路径；第四切片只加 facade/adapter，旧函数委托，不改变 `Kernel::register_agent` 参数和顺序。

- 风险：`AgentBuilder::build()` 行为变化会影响 integration tests 和 kernel e2e。
  控制：新增 `AgentSpec` snapshot/parity tests，验证同一 config 生成同一 agent manifest、permission、capabilities、llm options。

- 风险：Validation chain 过度设计。
  控制：第一版只实现已有校验等价规则，不接入真实 tool/skill registry 查询，不引入新依赖。

- 风险：Trace policy 变成空字段，没有实际约束。
  控制：本轮要求所有 `AgentSpec` 都带 `TracePolicy::Required` 默认值，registry facade 注册前检查该字段存在；后续 framework 消费迁移再把它接入 traced factory。

- 风险：deprecated API 太早删除或导致外部调用破坏。
  控制：旧 API 不删除，只在新入口稳定后标记 deprecated；兼容测试保留。

## 成功标准

- `AgentBuilder` 可产出 `AgentSpec`，旧 `build()` / `build_with_manifest()` 行为保持 1:1。
- `AgentSpec` 包含 name、capabilities、permission、llm options、prompt template、trace policy 等注册所需信息。
- Persona prototype clone + override 不修改原始 persona。
- 默认 validation chain 与现有 `AgentConfig::validate` 结果一致。
- 新 `MacacaSdk` facade 能通过 registry adapter 注册 agent。
- 旧 `register_from_config` / `register_from_file` 保留但可标记 deprecated，并委托新 facade。
- 不改变 app runtime、web start_server、kernel registration、trace/EventLog/SSE、task loop、driver、skill、MCP 行为。
- 不引入 application/workflow/driver/agent 名称硬编码。
