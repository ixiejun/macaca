# Macaca OS Plugin 服务完善 Superpowers Brainstorm

## 1. 背景

Macaca 已经完成 Route C 的基础微内核、系统服务化、模块化与 workspace topology 重构。当前 Plugin Runtime v0 已经具备：

- `macaca-proto` 中的 `PluginManifest`、`PluginRuntimeKind`、`PluginProvidedService`、`PluginPermission`、`PluginResource`、`PluginLifecycleEvent` 等协议模型。
- `macaca-runtime-host` 中的 `PluginRuntimeFacade`、`PluginManifestValidator`、`PluginHostFactory`、descriptor-only host 和 built-in adapter。
- `macaca-kernel` 中的 `PluginRegistry`，负责 plugin identity、lifecycle state、service/capability ownership 与 uninstall cleanup。
- OpenSpec `add-plugin-runtime-v0` 已明确 v0 是 descriptor/registry/lifecycle/validation first，不执行第三方代码。

这些能力奠定了地基，但还没有让 Plugin 成为 Macaca OS 的完整核心能力。当前缺口主要是：

- 没有完整安装源与本地插件仓库。
- 没有开发者 SDK / ABI / contract test kit。
- 没有可执行 plugin host，包括 WASM、process、remote proxy、native adapter 的安全执行边界。
- 没有统一 hook bus，无法让 plugin 参与 agent lifecycle、tool call、context build、memory ingest、gateway message、approval、trace、health 等关键节点。
- 没有 plugin config、secret、resource lease、permission admission、entitlement、签名验证的完整联动。
- 没有 Web / CLI 的 plugin 管理面。
- 没有将 Driver / Skill / MCP / Gateway / Memory / Context / LLM provider / Observability 等能力完整拉通为 plugin-provided capability。

## 2. 参考系统结论

### 2.1 OpenHarmony 对 Macaca Plugin 的启发

OpenHarmony 的系统能力不是随机散落的库，而是通过系统服务、能力声明、组件化、权限、生命周期、可裁剪模块形成 OS 级能力面。映射到 Macaca：

- Plugin 不应只是“扩展脚本”，而应是安装、注册、授权、运行、审计、升级、卸载的 OS capability package。
- Plugin 不应绕过 Service Registry；它提供的是 service/capability descriptor 和 capability implementation。
- Plugin 必须可裁剪。未安装某个 gateway / driver / memory / provider plugin 时，基础 OS 仍应运行。
- Plugin Runtime 不应进入 kernel。kernel 只保留注册表、身份、权限入口、trace/audit 和资源调度原语。

### 2.2 OpenClaw Plugin 系统可借鉴点

OpenClaw 的 plugin 生态成熟，值得借鉴：

- Manifest + package metadata 明确插件身份、兼容版本、contracts、activation、config schema、provider auth env vars。
- Plugin SDK 通过 `definePluginEntry` / channel/provider entry helpers 暴露窄接口，而不是让插件依赖内部模块。
- Runtime registry 统一管理工具、provider、channel、hooks、HTTP route、CLI、commands、middleware 等注册结果。
- Contract-first：插件声明 contracts，系统可以在不加载 runtime 的情况下发现能力归属。
- Channel plugin 将 provider-specific target grammar、DM policy、thread mapping、outbound delivery、approval capability 放在插件侧，core 保留通用 message tool 和 session 语义。
- Provider plugin 支持 auth choices、catalog、model support、runtime hooks，不需要 core 硬编码 provider。
- Hook runner 有错误策略、超时、优先级、阻断/观察语义，避免 hook 失控影响主流程。
- Bundled plugin 与 external plugin 有不同 trust policy，重能力仅对可信 bundled seam 开放。
- 测试体系覆盖 manifest contract、runtime registry、public surface、plugin SDK boundary、bundled plugin path、release checks。

需要避免的短板：

- TypeScript plugin SDK 规模很大，Macaca 不应一次性复制全部 surface。
- 很多 gateway/channel 细节如果直接搬进 Macaca core，会重新造成宏内核。
- 插件能力过多时，若没有 capability 分层和权限边界，会形成一个新的“大插件内核”。

### 2.3 Hermes Plugin 系统可借鉴点

Hermes 的 plugin 系统更轻量，但有几个实用设计：

- 插件来源分层：bundled、user、project、pip entry-point，后者覆盖前者。
- `plugin.yaml` + `register(ctx)` 简单清晰，小白开发者容易理解。
- PluginContext 是 Facade，插件通过它注册 tool、hook、command、platform、context engine、image provider，而不是直接改 core。
- `plugins.enabled` / `plugins.disabled` 支持显式启停，默认保守。
- install/update/remove/list CLI 体验完整，包含 git clone、manifest version、env prompt、after-install.md。
- `requires_env` 能直接驱动配置提示。
- Hook 点覆盖 tool、LLM、gateway dispatch、session lifecycle、approval lifecycle。
- 不同 kind：standalone、backend、exclusive、platform，解决“工具插件”和“单槽 provider 插件”的差异。

需要避免的短板：

- Python import 执行边界较弱，不适合 Macaca 的 OS 安全目标。
- manifest 较松散，权限、资源、trace schema、entitlement、签名、沙箱不够严格。
- 插件直接注册到全局 registry，长期容易产生隐式耦合。

## 3. 核心问题定义

Macaca Plugin 服务下一阶段要解决的问题不是“能不能装一个插件”，而是：

> 如何让 Plugin 成为 Macaca OS 上扩展系统能力的标准机制，同时保持微内核边界、服务化边界、可插拔、可审计、可替换、可裁剪和安全执行。

这要求 Plugin 服务至少具备以下系统级能力：

- 插件包发现、安装、校验、启停、升级、卸载。
- 插件 manifest schema 与 compatibility policy。
- 插件 SDK / ABI，使第三方不依赖内部 Rust crate。
- 插件 runtime host，支持 descriptor、built-in adapter、WASM、process、remote proxy 等模式。
- Plugin-provided service/capability 注册与调用。
- Hook bus，使插件安全参与系统关键执行点。
- Permission、resource、secret、config、entitlement、trace/audit 统一治理。
- Web / CLI / SDK 管理面。
- Contract tests 和 certification，保证第三方插件不会破坏 OS。

## 4. 设计模式候选

### 4.1 必选模式

- **Facade**：`PluginService` / `PluginRuntimeFacade` 对 Web、CLI、SDK、Application 暴露统一管理入口，隐藏 discovery、validation、host、registry、policy、trace 细节。
- **Abstract Factory**：按 runtime kind 创建 `DescriptorHost`、`BuiltInAdapterHost`、`WasmHost`、`ProcessHost`、`RemoteProxyHost`，调用方不依赖具体 host。
- **Adapter**：把现有 Driver、Skill、MCP、Gateway、Memory、Context、LLM provider、Observability 能力适配成 plugin-provided capability。
- **Strategy**：安装源、签名验证、权限策略、资源策略、hook failure policy、activation policy、entitlement policy、config source 都应可替换。
- **Chain of Responsibility**：manifest validation、signature verification、compatibility check、permission admission、resource admission、entitlement check、security scan 顺序执行。
- **State**：plugin lifecycle 必须保持 typed state machine，不允许随意跳转。
- **Observer / Event Bus**：plugin lifecycle、hook execution、capability call、health change、policy denial 都发 trace/audit event。
- **Proxy**：进程外、远端、WASM plugin 都通过 proxy 调用，不把实现对象暴露给 core。
- **Command**：install/start/stop/update/uninstall/call/hook 都应是 typed command，便于审计、重试、权限检查。
- **Specification**：manifest、permission、resource、compatibility、hook contract、capability descriptor 都用规则对象验证。

### 4.2 谨慎使用模式

- **Composite**：一个 plugin 提供多个 capability，适合 manifest/provided services，但不能让一个 plugin 聚合成不可治理的 mega-plugin。
- **Builder**：适合 manifest builder、plugin test fixture、SDK 注册 API，但避免过度复杂。
- **Null Object**：缺失可选插件时返回 unavailable provider，保持系统可裁剪。
- **Memento**：plugin registry snapshot、health snapshot、activation snapshot 适合诊断和回滚。

## 5. 可选方案

### 方案 A：只增强现有 descriptor runtime，不执行第三方插件

做法：

- 完善 manifest、config、secret、Web/CLI 管理、built-in descriptor。
- 所有能力仍由内置 Rust service 实现。
- Plugin 只作为“能力描述与开关”。

优点：

- 风险最低。
- 不涉及沙箱、外部进程、WASM runtime。
- 容易保持现有测试稳定。

缺点：

- Plugin 仍不是完整能力，只是 registry metadata。
- 第三方生态无法成立。
- 无法满足 gateway/driver/memory/provider 可替换需求。

结论：只能作为短期补丁，不应作为目标路线。

### 方案 B：优先实现 process / native plugin，快速获得真实能力

做法：

- 插件用本地进程或动态库执行。
- CLI 安装插件后直接启动插件 host。
- 插件通过 IPC 注册 tools/hooks/services。

优点：

- 能快速支持多语言。
- 对复杂 gateway/driver/observability 插件友好。
- 类似 Hermes 的易用体验。

缺点：

- 安全风险大，必须先有权限、secret、resource、签名、sandbox、process supervisor。
- native ABI 稳定难度高。
- 插件崩溃、资源泄漏、后台驻留会影响 7x24 OS。

结论：适合作为第二批 runtime host，但不应第一个落地。

### 方案 C：WASM-first Plugin Runtime

做法：

- 插件编译为 WASM component / WASI module。
- 通过 Macaca Plugin ABI 调用 host functions。
- capability 注册、hook、tool invocation 都走 host ABI。

优点：

- 安全边界强。
- 语言无关。
- 与 Application ABI 路线一致。
- 更符合 OS 级生态。

缺点：

- 前期工程量大。
- WASM 生态对复杂 gateway、浏览器、系统进程、OAuth callback 支持需要 host bridging。
- 小白开发者体验不如脚本插件直接。

结论：推荐作为中长期主路线，但需要先补足 install/config/SDK/contract。

### 方案 D：Remote Proxy Plugin Runtime

做法：

- 插件可以是本地或远程 service。
- Macaca 只持有 manifest、endpoint、auth、capability descriptors。
- 调用通过 IPC / HTTP / gRPC / MCP-like transport proxy。

优点：

- 适合企业系统、远端 gateway、云端 memory、外部 tool provider。
- 插件进程隔离天然更强。
- 便于用户替换第三方系统。

缺点：

- 需要认证、重试、超时、健康检查、契约版本、数据边界。
- 离线体验弱。

结论：应与 WASM 并列作为核心 runtime kind，尤其适合 Memory、Gateway、Enterprise Tool、Observability。

### 方案 E：分层 Plugin Fabric，先控制面，后执行面

做法：

- 第一阶段做 Plugin Control Plane：安装源、本地仓库、manifest、validation、config/secret、activation、health、Web/CLI。
- 第二阶段做 Plugin Capability Plane：capability registry、hook bus、tool/provider/channel/context/memory 注册 API。
- 第三阶段做 Plugin Execution Plane：WASM host、process host、remote proxy host。
- 第四阶段做 Plugin Ecosystem Plane：SDK、contract test kit、certification、Store/Entitlement、版本升级、签名、分发。

优点：

- 渐进式，符合当前 Route C。
- 每个阶段都有可验证价值。
- 避免一上来写不安全执行器。
- 能同时吸收 OpenClaw 的 contract-first 和 Hermes 的易用安装体验。

缺点：

- 看起来阶段较多。
- 短期仍无法运行任意第三方代码。

结论：推荐。它最符合 Macaca 作为 Agent OS 的长期架构与当前代码状态。

## 6. 推荐路线

推荐采用 **方案 E：分层 Plugin Fabric，先控制面，后执行面**。

核心原则：

- Plugin 是 OS capability package，不是随意脚本。
- Kernel 只管 plugin identity、descriptor ownership、lifecycle invariant，不执行 plugin。
- Runtime-host 管 plugin host factory、lifecycle supervisor、sandbox/proxy、health。
- Services 管 plugin-provided capability 的业务接口，不把业务塞进 plugin runtime。
- SDK 只暴露窄接口，不暴露内部 crate。
- Web/CLI 是 thin shell，通过 Plugin Service / SDK client 管理插件。
- 所有插件行为必须可 trace、可审计、可关闭、可卸载、可降级。

## 7. 推荐能力分期

### Phase P0：Plugin v0 差距审计与现有任务收敛

目标：

- 审计 `add-plugin-runtime-v0` 的未完成项，例如 built-in descriptor canonical facade、lifecycle trace 是否真正进入 event log、deprecated bypass anchor。
- 审计 ServiceRuntime、Store/Entitlement、Driver/Skill/MCP/Gateway/Memory 与 plugin 的当前接入点。
- 明确哪些能力已有骨架，哪些只是文档声明。

输出：

- 一个差距审计文档。
- 一个 OpenSpec proposal 清单。

风险：

- 如果不先审计，后续会重复实现或与现有 serviceization 冲突。

### Phase P1：Plugin Control Plane v1

目标：

- 本地插件仓库：bundled、user、project、store-cache、dev-link。
- 安装源抽象：local dir、git、archive、store、remote catalog。
- manifest loader 与 version compatibility。
- enable/disable、activation policy、startup policy、health snapshot。
- config schema、env/secret requirement、after-install guidance。
- CLI/Web 管理接口：list、inspect、install、enable、disable、remove、health。

借鉴：

- Hermes 的 install/update/remove/list、env prompt、after-install。
- OpenClaw 的 manifest contract、activation、config schema、contract discovery。

设计模式：

- Strategy：install source、activation policy、compat policy。
- Chain of Responsibility：load -> parse -> validate -> signature -> compatibility -> admission。
- Facade：`PluginControlService`。
- Memento：registry snapshot。

### Phase P2：Plugin Capability Registry v1

目标：

- 插件可声明并注册 capability types：tool、hook、driver、gateway、skill、mcp、memory、context、llm-provider、observability、http-route、cli-command。
- Contract-first：系统可以在不启动 runtime 的情况下知道 capability owner。
- 统一 descriptor schema：输入 schema、输出 schema、permission hints、resource hints、trace schema、visibility、activation requirement。
- capability conflict policy：同名 tool、provider slot、exclusive memory/context provider、gateway channel route 等冲突可解释。

借鉴：

- OpenClaw 的 `contracts.tools`、provider/channel contracts。
- Hermes 的 kind：backend、exclusive、platform。

设计模式：

- Composite：一个 plugin 多 capability。
- Specification：capability contract validation。
- Registry：descriptor ownership。
- Null Object：missing optional capability。

### Phase P3：Plugin Hook Bus v1

目标：

- 提供安全 hook 点：before/after tool call、before prompt build、context assemble、memory ingest、pre/post LLM call、session start/end、gateway message received/sending、approval request/response、task lifecycle、application lifecycle。
- Hook 类型分层：observer、mutating、blocking、approval。
- Hook failure policy：fail-open / fail-closed / timeout / priority。
- Hook result schema：allow、block、rewrite、require approval、contribute context、emit audit。
- Hook 执行必须 trace，不能无限阻塞。

借鉴：

- OpenClaw hook runner 的 failure policy、timeout、priority、blocking semantics。
- Hermes 的 `VALID_HOOKS` 轻量覆盖面。

设计模式：

- Observer：事件广播。
- Chain of Responsibility：按优先级执行 hook。
- Command：hook invocation command。
- Strategy：failure policy。

### Phase P4：Plugin SDK / ABI v1

目标：

- 为插件开发者提供稳定 SDK，不依赖内部 crate。
- SDK API 包含 register tool/provider/channel/hook/service/config/secret/health。
- 提供 contract test kit、fixture builder、schema validation、compat test。
- 支持至少两类开发体验：
  - Rust SDK：适合本仓内置或高性能插件。
  - WASM Component SDK：长期主路线。
  - Remote Proxy SDK：适合企业/云端服务。

借鉴：

- OpenClaw `definePluginEntry` 和 narrow subpath SDK。
- Hermes `PluginContext` 的易用 Facade。

设计模式：

- Facade：`PluginContext` / `PluginApi`。
- Builder：manifest and fixture builder。
- Adapter：SDK adapter to service contracts。

### Phase P5：WASM Plugin Host v1

目标：

- 插件可通过 WASM component 安全执行。
- Host functions 仅暴露 capability-scoped API：trace、config、secret resolve、service call、emit event、register descriptor。
- 沙箱限制 filesystem/network/process，全部通过 resource lease 和 permission admission。
- 支持 startup、shutdown、hook invoke、tool invoke、health probe。

设计模式：

- Abstract Factory：`WasmPluginHostFactory`。
- Proxy：WASM host proxy。
- Resource Manager：resource lease。
- State：runtime lifecycle。

风险：

- WASM host 与 ABI 设计需要谨慎，不能过早冻结过多接口。

### Phase P6：Process / Remote Proxy Plugin Host v1

目标：

- 支持本地外部进程和远端插件服务。
- 进程 supervisor 负责 spawn、restart、timeout、stdio/rpc framing、health、shutdown。
- Remote proxy 负责 auth、TLS、timeout、retry、circuit breaker、schema negotiation。

适用：

- Gateway、browser/desktop driver、enterprise memory、observability、heavy provider。

设计模式：

- Proxy。
- Circuit Breaker。
- Supervisor。
- Strategy：transport selection。

### Phase P7：Store / Entitlement / Signature Integration

目标：

- plugin package 与 Store/Entitlement 对齐。
- 签名验证、开发者身份、license、subscription、paid capability。
- 安装前 `before_install` hook 与安全扫描。
- 加密包或付费包不泄漏实现。

设计模式：

- Chain of Responsibility：signature -> entitlement -> scan -> install。
- Policy / Specification。
- Audit Log。

### Phase P8：Web / CLI Plugin Management

目标：

- Web UI 展示 plugin list、status、capabilities、permissions、resources、health、logs、trace、config。
- CLI 支持 install/list/inspect/enable/disable/remove/doctor/test。
- 开发者体验：scaffold、validate、package、run-contract-tests。

设计模式：

- Thin Shell：Web/CLI 只走 SDK client。
- Facade：Plugin Service Client。

## 8. 关键上下游影响

### 8.1 macaca-proto

需要扩展：

- Plugin install source、package metadata、config schema、secret requirement、capability contract、hook contract、host ABI、health report、diagnostics。

风险：

- proto 过早膨胀。应按 phase 增量添加，保持 provider-neutral。

### 8.2 macaca-kernel

保持：

- 只拥有 registry invariant、identity、lifecycle、ownership、permission gateway。

禁止：

- 不执行 plugin code。
- 不实现 gateway/driver/memory/provider behavior。

### 8.3 macaca-runtime-host

需要成为插件执行与监督的主承载：

- host factory。
- WASM/process/remote proxy。
- resource lease。
- lifecycle supervisor。
- health probe。

### 8.4 macaca-ipc / ServiceRuntime

需要支撑：

- plugin-provided service call。
- out-of-process plugin transport。
- timeout/retry/correlation id/trace id。

### 8.5 Driver / Skill / MCP / Gateway / Memory / Context / LLM

需要逐步迁移：

- 内置实现继续存在，但作为 built-in plugin descriptor 或 built-in plugin package。
- 新实现优先通过 plugin capability 接入。
- 直接 runtime API 标记 deprecated，不删除，便于查找迁移。

### 8.6 Store / Entitlement

需要成为 plugin 安装和付费能力的治理层：

- signature。
- developer identity。
- package source。
- entitlement。
- paid capability admission。

### 8.7 Web / CLI

需要只作为 thin shell：

- 不直接读 plugin directory。
- 不直接启动 plugin process。
- 通过 Plugin Service / SDK client 获取状态和执行管理命令。

## 9. 风险与缓解

| 风险 | 后果 | 缓解 |
| --- | --- | --- |
| 过早执行第三方代码 | 安全事故、资源泄漏、7x24 稳定性下降 | 先做 control plane 与 capability contracts，再做 WASM/process host |
| Plugin Runtime 变成宏内核 | 又把 gateway/driver/provider 逻辑塞回中心 | Runtime-host 只做 host/supervisor/proxy，业务在 service/plugin |
| SDK 过大 | 开发者难用，维护成本高 | 按 capability 拆窄 subpath SDK，优先小核心 |
| Hook 失控 | 主流程卡死或行为不可预测 | timeout、priority、failure policy、typed result、trace |
| 权限成为装饰 | 插件可绕过资源边界 | 所有 capability call 必须通过 permission/resource admission |
| manifest 兼容性混乱 | 插件生态升级困难 | manifest version、compat range、contract tests |
| 硬编码 provider/app 名称 | 破坏通用 OS 目标 | 只允许 capability/category/id，不允许业务分支 |
| Store 和 Plugin 双轨 | 安装、签名、entitlement 逻辑重复 | Plugin package 归 Store/Entitlement 治理，Plugin Runtime 只运行已授权包 |
| Web/CLI 重度耦合 | 前端/命令行变成第二个 runtime | Thin shell，只调用 Plugin Service client |

## 10. 建议下一步

建议下一步先做 `write-plan`，计划名称：

`2026-05-11-plugin-service-enrichment-plan.md`

建议该 plan 拆成 4 个 OpenSpec 提案，而不是一个巨大提案：

1. `add-plugin-control-plane-v1`
   - 插件仓库、安装源、manifest loader、enable/disable、config/secret requirement、health snapshot、CLI/Web service API。
2. `add-plugin-capability-registry-v1`
   - capability contracts、contract-first discovery、conflict policy、built-in service adapter canonicalization。
3. `add-plugin-hook-bus-v1`
   - typed hook bus、failure policy、timeout、priority、trace/audit、核心 hook 点。
4. `add-plugin-sdk-and-hosts-v1`
   - SDK facade、contract test kit、WASM host skeleton、process/remote proxy skeleton。

如果需要更保守，也可以先只做第一个提案 `add-plugin-control-plane-v1`，但从架构完整性看，四个提案需要在同一批设计中统一边界，避免后续返工。

## 11. Brainstorm 结论

Macaca Plugin 服务的目标不是复制 OpenClaw 或 Hermes，而是吸收两者优点并提升到 OS 级架构：

- 学 OpenClaw 的 contract-first、SDK boundary、provider/channel/tool/hook registry、严格测试与生态分发。
- 学 Hermes 的简单安装体验、`PluginContext` 易用 Facade、env prompt、轻量 hook 和本地开发友好。
- 保持 OpenHarmony 式系统能力服务化、可裁剪、权限治理、生命周期与统一能力注册。

推荐路线是分层 Plugin Fabric：先把控制面、能力注册、hook bus、SDK/ABI、host runtime、Store/Entitlement 逐层补齐。这样 Plugin 才能从当前骨架升级为 Macaca OS 的核心完整能力，同时不牺牲微内核边界和 7x24 基础设施稳定性。
