# Macaca Application 生态与 SDK 完整化 Brainstorm

## 背景

本次 brainstorm 针对 `docs/openharmony-microkernel-architecture-for-macaca-agent-os.md` 第 5 节“Macaca OS 的目标能力模型”。当前 Macaca Application 仍以 YAML 配置包为主要开发方式，已有 `macaca-app`、`macaca-sdk`、`macaca-proto` 和 `macaca-runtime-host` 的应用服务化骨架，但还没有形成真正面向第三方开发者的完整应用生态。

目标不是把 demo YAML application 做得更丰富，而是让 Macaca 具备长期可演进的 Application Platform：

- 支持声明式 YAML app、WASM component app、GenUI app、headless service app、hybrid app、paid/store-distributed app、plugin-enhanced app。
- 支持开发者通过稳定 SDK、ABI、manifest、toolchain、testkit、packaging、compatibility checker 和 capability contract 构建应用。
- 支持上层应用通过公开标准调用 Task、LLM、Memory、Context、Driver、Skill、MCP、Plugin、Store、Payment、Web3/EVM、GenUI 等系统服务。
- 保持微内核边界：Application Framework 拥有应用模型，Kernel 只拥有系统不变量，Runtime Host 拥有服务宿主，SDK 只暴露 developer-facing facade，Web/CLI 只是 shell。

必须遵守：

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/design_patterns.md`

## 当前代码现状

- `macaca-app/src/model.rs` 已有 `AppManifest`、`AppLayer`、`UiType`、agent/workflow/resource/context 配置，但仍偏 YAML/agent 声明模型。
- `macaca-app/src/package.rs` 已能把 YAML app 适配成 Route C `PackageDescriptor`，但 package manifest 还不是开发者的主入口。
- `macaca-app/src/abi.rs` 已有 `ApplicationAbiAdapter`、YAML adapter、WASM metadata-only adapter、`ApplicationAbiInstance`，但尚未有真实 WASM component host。
- `macaca-proto/src/application_abi.rs` 已定义 ABI v0 exports/imports、host command、lifecycle、render/storage/payment/service call 等数据契约。
- `macaca-sdk/src/application.rs` 已有 `ApplicationAbiBuilder` 和 host command builder，但仍不是完整 Application SDK。
- `macaca-sdk/src/application_client.rs` 已有 `SystemApplicationClient`，主要服务 Web/CLI shell 调 Application Service，不是 app developer SDK。
- S7 已把 Application Framework 生命周期服务化，但 Web 仍保留直接 manifest reads、registry fallback 和兼容执行路径。

## 外部生态可借鉴点

### macOS / Apple 平台

可借鉴点：

- Apple 同时保留成熟 imperative 框架（AppKit/UIKit）和声明式 SwiftUI，说明 Macaca 不应只押注一种 app 形态。YAML、Rust SDK、WASM SDK、GenUI DSL 可以并存。
- App lifecycle、delegate/scene/window、entitlements、bundle manifest、sandbox、App Store 审核构成一条完整开发链，而不是只有 runtime API。
- SDK 需要提供开发者能理解的高层 API，同时底层仍通过 stable ABI 与系统交互。

不应照搬：

- 不采用封闭平台模型。Macaca 必须支持多语言、多 runtime、多分发渠道和可替换系统服务。
- 不让 UI framework 直接拥有 OS 业务语义。GenUI 是 UI service，不是 application runtime 的全部。

参考：Apple Developer AppKit / UIKit / SwiftUI 文档强调平台 UI framework、lifecycle 与开发工具链的组合。

### Windows / Windows App SDK

可借鉴点：

- Windows App SDK 把开发者 API 与 OS release 解耦，Macaca SDK 也应支持独立版本、兼容矩阵和 feature detection，而不是要求 app 跟随 OS 内部 crate。
- App lifecycle、activation、packaging、capability、deployment 是应用生态底座。Macaca 需要 `macaca app new/build/package/check/test/publish` 这种完整工具链。
- SDK 应覆盖 GUI、headless、service、extension、activation protocol，而不是只覆盖聊天入口。

不应照搬：

- 避免历史包袱导致 API 碎片化。Macaca 现在仍早期，应把 manifest、ABI、SDK、service client 的职责一次性分清。

参考：Microsoft Windows App SDK 官方文档说明 SDK 通过 NuGet 与 OS 解耦，并覆盖 runtime behavior、activation、packaging 等能力。

### Linux / freedesktop / Flatpak / Portal

可借鉴点：

- Flatpak 的 manifest、runtime、sandbox、portal 模型适合 Macaca：应用不能直接访问宿主资源，只能通过 portal/service request 获取受控能力。
- freedesktop desktop file、AppStream metadata、portal API 说明“应用包 metadata + runtime capability + sandbox access broker”比单个配置文件更重要。
- Macaca Application 应通过 Host ABI / Service Portal 调 Task、Driver、Memory、Storage、Network、Payment，而不是直接依赖内部 crate。

不应照搬：

- Linux 桌面生态过度碎片化。Macaca 应提供统一 manifest、统一 capability vocabulary、统一 SDK testkit，避免每个服务自定义一套开发入口。

参考：Flatpak / XDG Desktop Portal 官方文档强调 sandboxed app bundle、manifest、portal-mediated resource access。

### 微信小程序

可借鉴点：

- 小程序成功点在于低门槛：清晰目录结构、全局配置、页面配置、生命周期、组件模型、API namespace、开发者工具、审核和发布流程。
- Macaca 需要给小白开发者一个“最短路径”：不懂 Rust/WASM 也能用 YAML/TypeScript-like DSL/SDK template 做出可运行 app。
- 配置与代码应分层：manifest 声明页面、能力、权限、入口，代码只写业务逻辑，系统统一提供能力 API。

不应照搬：

- 不能把 Macaca 锁进单一 JS 小程序模型。Macaca 是 Agent OS，应支持多语言 WASM、后台 agent/service、GenUI、插件、MCP 和企业自动化。

参考：微信/小程序生态的 `app.json`、页面生命周期、开发者工具和平台 API namespace 模型。

### OpenHarmony

可借鉴点：

- Ability / Stage 模型把 UIAbility、ExtensionAbility、WindowStage、配置、生命周期和系统 capability 分清，适合 Macaca 设计 Application Ability Model。
- OpenHarmony 文档强调 application framework、ability framework、多语言 API、ArkUI 和系统能力 kit。Macaca 也需要 `Application Kit`、`Agent Ability Kit`、`GenUI Kit`、`Service Capability Kit`、`Store Kit`。
- Ability 是 app 组件，不是整个 app。Macaca 应支持一个 application 内有多个 ability：agent ability、UI ability、headless ability、scheduled ability、gateway ability、plugin extension ability。

不应照搬：

- 不绑定 ArkTS/ArkUI。Macaca 的 ABI 应保持 provider-neutral、language-neutral、renderer-neutral。

参考：OpenHarmony overview 与 Ability/ArkUI 相关文档强调应用框架、组件模型和多语言系统 API。

## 核心设计问题

### 问题 1：Application 到底是什么

候选定义：

- Application 是可安装、可启动、可授权、可审计、可升级的软件包。
- Application 可以包含一个或多个 Ability。
- Ability 是可被 OS 调度和交互的应用组件，例如 AgentAbility、UiAbility、HeadlessAbility、GatewayAbility、ScheduledAbility、PluginExtensionAbility。
- Application 不直接拥有系统服务实现，只声明依赖和权限，并通过 SDK/ABI 调用服务。

这比当前 YAML app 更通用，也比 plugin 更面向最终用户。

### 问题 2：SDK 应面向谁

SDK 至少要分三层：

- Developer SDK：给应用开发者写业务逻辑、声明能力、调用服务、渲染 GenUI、处理事件。
- Packaging SDK：给构建工具生成 manifest、ABI descriptor、package descriptor、signature metadata、compatibility report。
- Shell SDK：给 Web/CLI/Gateway 调系统 Application Service，目前已有 `SystemApplicationClient` 属于这一层。

当前 `macaca-sdk/src/application.rs` 更接近 ABI helper，`application_client.rs` 更接近 shell SDK，缺少真正的 Developer SDK。

### 问题 3：是否继续以 YAML 为中心

YAML 必须继续一等支持，但不应继续作为唯一中心。

建议把 YAML 定位为 Declarative Application Profile：

- YAML app 是 package runtime kind 的一种。
- YAML 通过 Adapter 适配到统一 Application Manifest v1、Package Descriptor、ABI Declaration。
- WASM/Hybrid/GenUI/headless app 使用相同 manifest 和 ABI，不走 YAML 特权路径。

### 问题 4：如何避免强耦合

禁止：

- SDK 依赖 `macaca-app` runtime concrete type。
- Application 依赖 `macaca-web`、`macaca-runtime-host`、`macaca-kernel` 内部类型。
- Web/CLI 解释 app workflow、permission、payment、runtime kind 的核心语义。
- Kernel 根据 app name、workflow name、driver name、business name 做分支。

允许：

- `macaca-proto` 承载 provider-neutral ABI、manifest、service command/result、capability vocabulary。
- `macaca-sdk` 提供 builders、typed clients、testkit、manifest helpers。
- `macaca-app` 拥有 Application Framework runtime/admission/loader/ability model。
- `macaca-runtime-host` 托管 Application Service provider 和 WASM/headless host。

## 设计模式候选

### Option A：继续扩展 YAML Application

做法：

- 在 `AppManifest` 上继续加字段：ui、permissions、services、store、wasm、plugin dependencies。
- SDK 只提供 YAML builder 和 validator。

优点：

- 改动最小。
- 对当前 demo app 兼容性最好。
- 短期能快速补齐字段。

风险：

- YAML 会变成新的宏内核入口，所有能力都被塞进一个大 manifest。
- WASM/GenUI/headless/hybrid 都被迫适配 YAML 语义，长期不可扩展。
- 不能形成多语言 SDK 和真正 app ecosystem。

结论：不建议作为主路线，只能作为兼容 adapter。

### Option B：Application Package + Manifest v1 优先

做法：

- 定义 `ApplicationManifestV1` 作为 app package 的事实来源。
- YAML app 被转换为 `ApplicationManifestV1`。
- WASM、GenUI、headless、hybrid 都从同一 manifest 进入 Application Framework。
- SDK 重点提供 manifest builder、package builder、compatibility checker、testkit。

适用设计模式：

- Adapter：YAML 转 Application Manifest v1。
- Builder：SDK 构建 manifest/package/permission/service declaration。
- Specification：兼容性、权限、ABI、service dependency 验证。
- Memento：package compatibility report 和 app snapshot。

优点：

- 能把 YAML 从中心降级为一种 package profile。
- 对 Store、签名、entitlement、compatibility checker 天然友好。
- 不要求一次实现真实 WASM 执行。

风险：

- 如果只做 manifest，不做 Developer SDK，开发体验仍然像“写配置”。
- 需要迁移 Web/CLI 读取 raw `AppManifest` 的路径。

结论：必要基础，但不足以单独完成生态。

### Option C：WASM Component ABI 优先

做法：

- 把 WASM component 作为 Application 运行时主线。
- SDK 以 WIT/ABI 为中心，Rust/Go/TS/Python 编译到 WASM。
- Host imports 全部通过 capability/service portal。

适用设计模式：

- Bridge：WASM guest 与 Macaca host service 隔离。
- Command：host import request 是 typed command。
- Proxy：SDK guest binding 是 host service proxy。
- Null Object：未安装 WASM runtime 时返回 structured unavailable。

优点：

- 最符合多语言、安全沙箱、长期 ABI 稳定目标。
- Application 与内部 Rust crate 解耦彻底。
- 能形成接近 OS 的应用模型。

风险：

- 一次性实现成本高。
- WASM component toolchain、WIT、bindings、host runtime、async service call 都需要严肃设计。
- 如果过早强制 WASM，会伤害小白开发者和 YAML 快速开发路径。

结论：必须作为长期核心，但不应第一步就只做 WASM。

### Option D：Ability Model 优先

做法：

- 借鉴 OpenHarmony，把 app 内部拆成多个 ability：
  - `AgentAbility`
  - `UiAbility`
  - `HeadlessAbility`
  - `ScheduledAbility`
  - `GatewayAbility`
  - `PluginExtensionAbility`
  - `StoreEntitledAbility`
  - `Web3Ability`
- 每个 ability 有 lifecycle、permissions、required services、activation routes、trace policy。
- YAML/WASM/Hybrid 只是 ability 的 implementation kind。

适用设计模式：

- Composite：Application 包含多个 Ability。
- State：每个 Ability 有独立 lifecycle。
- Strategy：不同 runtime kind 使用不同 AbilityHost。
- Visitor：compatibility checker 遍历 app/ability/service/permission。
- Specification：ability admission。

优点：

- 能自然支持 GUI、headless、agent、gateway、scheduled、plugin extension。
- 不把 application 简化为一个聊天入口。
- 与 OpenHarmony Stage/Ability 思想接近，适合 OS 生态。

风险：

- 如果没有 package/SDK/toolchain 配合，会变成抽象模型而非开发者能力。
- 需要避免 ability model 过度复杂，初版必须有清晰最小集合。

结论：强烈建议作为 Application Framework 的领域模型核心。

### Option E：分层 Application Platform，Package + Ability + SDK + ABI 一起演进

做法：

- 以 Application Package Manifest v1 作为事实来源。
- 以 Ability Model 表达应用组件。
- 以 SDK Kits 提供开发体验。
- 以 WASM ABI 作为长期二进制边界。
- 以 YAML Adapter 继续一等支持声明式 agent app。
- 以 Application Service 作为 shell-facing lifecycle/control plane。

分层：

- `macaca-proto`：Application manifest v1、Ability descriptor、ABI/service command/result、capability vocabulary。
- `macaca-app`：Application Framework、manifest loader、ability registry、admission specs、compatibility checker、YAML/WASM/headless adapters。
- `macaca-sdk`：Developer SDK、manifest/package builders、host service proxies、testkit、examples。
- `macaca-runtime-host`：Application Service provider、Ability host supervisor、WASM/headless host skeleton、trace/policy decorators。
- `macaca-web` / `macaca-cli`：thin shell、GenUI renderer、developer command adapter，不定义 app semantics。

适用设计模式：

- Facade：Developer SDK 和 SystemApplicationClient 隐藏底层复杂度。
- Adapter：YAML、WASM metadata、legacy app manifest、future package store 都适配到统一 model。
- Bridge：Application ABI 与 System Service Runtime 分离。
- Abstract Factory：按 runtime kind 创建 AbilityHost。
- Builder：manifest/package/ability/permission/service declaration。
- Command：host import、UI event、activation、service call。
- Composite：Application -> Abilities -> Capabilities。
- Specification：admission、compatibility、permission、ABI、entitlement。
- Observer：trace/audit/event stream。
- State：application/ability lifecycle。
- Memento：snapshot、checkpoint、compat report。
- Null Object：missing optional runtime/service/module returns unavailable。

优点：

- 同时解决长期架构和短期兼容。
- 不牺牲 YAML，也不让 YAML 限制未来。
- 能真正支持百花齐放的 application ecosystem。
- 与 Route C 微内核边界一致。

风险：

- 范围大，需要拆分阶段和 OpenSpec。
- SDK 和 ABI 边界如果设计不好，会形成新的强耦合。
- Ability model 若过度复杂，会增加开发者学习成本。

结论：推荐方案。

## 推荐方向

推荐采用 Option E：分层 Application Platform。

核心判断：

- Macaca 是 Agent OS，不是 Chat UI，也不是 YAML runner。
- Application 必须是可安装、可授权、可审计、可扩展的软件包。
- SDK 必须让开发者不用理解内部 crate，就能开发 GUI/headless/agent/hybrid app。
- ABI 必须长期稳定，并允许 Rust/Go/TS/Python 等语言通过 WASM component 接入。
- YAML 必须继续一等支持，但只是 Application Package 的一种 declarative profile。
- Web/CLI/Gateway 只能调用 Application Service / SDK facade，不能继续解释应用语义。

## 关键能力清单

### 1. Application Manifest v1

需要支持：

- package metadata：id、version、developer、signature、min_os_version、abi_version。
- runtime：yaml、wasm_component、native_builtin、headless、hybrid。
- abilities：agent、ui、headless、scheduled、gateway、plugin_extension。
- services：required services、optional services、capability contracts。
- permissions：storage、network、llm、memory、driver、skill、mcp、payment、web3、ui。
- ui：GenUI surface、component bundle、event schema、theme policy。
- commerce：license、subscription、metering、store_required。
- compatibility：feature gates、SDK version、ABI version、OS version。

### 2. Ability Model

最小集合：

- `AgentAbility`：声明 agent roles、tools、skills、memory/context policy、entry behavior。
- `UiAbility`：声明 GenUI surface、render event、UI action handler。
- `HeadlessAbility`：声明 background task、daemon、automation entry。
- `ScheduledAbility`：声明定时触发和 heartbeat，不把调度写死在 app。
- `GatewayAbility`：声明外部入口适配需求，由 Gateway Service/Plugin 实现。
- `ExtensionAbility`：声明 application-provided extension points。

### 3. Developer SDK Kits

建议 SDK kits：

- `ApplicationKit`：manifest、ability、lifecycle、activation。
- `AgentKit`：agent ability、task goal、tool/capability call、context/memory access。
- `GenUiKit`：surface builder、event schema、action handler。
- `ServiceKit`：typed service client proxy。
- `StorageKit`：scoped kv、artifact、secret ref，不暴露真实 secret。
- `TraceKit`：structured trace、span、audit event。
- `PermissionKit`：capability request、approval state。
- `StoreKit`：entitlement、metering、license status。
- `TestKit`：host simulation、contract tests、compatibility tests。

### 4. WASM Component ABI

长期需要：

- WIT 或等价 ABI schema。
- guest bindings：Rust first，然后 TypeScript/Go/Python。
- host imports：service_call、trace_emit、storage_get/set、ui_render、payment_intent、permission_request。
- host exports：init、start、handle_event、render、pause、resume、shutdown、upgrade。
- structured unavailable：缺 WASM runtime、缺 optional service、权限拒绝必须可审计。

### 5. Toolchain

需要 CLI/SDK 支持：

- `macaca app new`
- `macaca app check`
- `macaca app test`
- `macaca app build`
- `macaca app package`
- `macaca app run`
- `macaca app publish`

这些命令应调用 SDK/package checker/Application Service，不应在 CLI 内部复制 runtime 语义。

### 6. Compatibility 与认证

需要：

- manifest schema validation。
- ABI compatibility validation。
- permission/capability validation。
- service dependency validation。
- optional module unavailable tests。
- trace-required tests。
- secret redaction tests。
- store entitlement tests。
- SDK examples contract tests。

## 风险与控制

### 风险 1：Application Platform 变成新宏内核

控制：

- Application Framework 只拥有 app/ability/package/lifecycle/admission。
- Task、LLM、Memory、Driver、Skill、MCP、Plugin、Store、Payment、Web3/EVM 仍通过 service client 调用。
- 禁止在 `macaca-app` 中构造 provider 或调用 provider concrete type。

### 风险 2：SDK 过度绑定内部 Rust crate

控制：

- Developer SDK 只依赖 `macaca-proto` 和 SDK facade contracts。
- app developer 不依赖 `macaca-web`、`macaca-kernel`、`macaca-runtime-host`。
- WASM guest SDK 通过 generated bindings 调 host imports。

### 风险 3：Ability Model 过度设计

控制：

- 初版只实现最小 ability descriptor 和 metadata/admission。
- 真实执行 host 分阶段落地。
- YAML app 自动映射到 `AgentAbility`，不要求开发者立即理解全部 ability。

### 风险 4：WASM runtime 实现成本过高

控制：

- 第一阶段保留 metadata-only WASM adapter 和 structured unavailable。
- 先完成 ABI/WIT/SDK/testkit，再实现真实 host。
- 允许 YAML/Hybrid 继续跑生产路径。

### 风险 5：开发者体验不如小程序

控制：

- 必须提供模板、脚手架、examples、testkit、错误诊断。
- SDK builder 不应要求开发者手写大量 JSON/YAML。
- `macaca app check` 必须给出可操作修复建议。

### 风险 6：Web/CLI 继续读取 raw manifest

控制：

- 新路径必须通过 Application Service sanitized views。
- 旧 raw manifest reads 标记 deprecated，只作为迁移锚点。
- dependency gate 证明 direct edge 消失前，不删除 allowlist 行。

## 建议的后续计划方向

如果进入 write-plan，建议拆成 6 个阶段：

1. Application Manifest v1 与 Ability Descriptor 基线。
2. SDK ApplicationKit / AbilityKit / Manifest Builder / TestKit。
3. YAML app adapter 迁移到 Manifest v1 + AgentAbility。
4. Application Service sanitized metadata view，替代 Web raw manifest reads。
5. WASM Component ABI/WIT skeleton + guest SDK scaffold + unavailable-safe host。
6. GenUI/Headless/Store/Plugin-enhanced app fixtures 与 certification tests。

每个阶段都应先创建 OpenSpec proposal / design / tasks / spec，再实施。

## 参考来源

- Apple Developer Documentation: AppKit / UIKit / SwiftUI application framework and UI lifecycle.
- Microsoft Learn: Windows App SDK, app lifecycle, activation, packaging, SDK decoupled from OS release.
- Flatpak / freedesktop / XDG Desktop Portal documentation: sandboxed app bundle, manifest, portal-mediated access.
- 微信小程序开发文档体系：`app.json`、页面配置、生命周期、开发者工具、平台 API namespace。
- OpenHarmony docs: application framework、ability framework、ArkUI、Stage/Ability model、多语言系统 API。
