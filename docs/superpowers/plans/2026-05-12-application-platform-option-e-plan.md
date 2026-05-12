# 分层 Application Platform 实施计划

## 1. 目标

基于 `2026-05-11-application-ecosystem-sdk-brainstorm.md` 选择 **Option E：分层 Application Platform**。

本计划把 Macaca Application 从“YAML 配置应用 + 服务化生命周期骨架”升级为真正面向第三方开发者的 Application Platform。完成后，Macaca 应能够支撑长期应用生态，而不是只支撑 demo application：

- YAML 配置应用继续一等支持，但被收敛为统一 Application Manifest v1 / Ability Model 的 declarative profile。
- 开发者可以通过 SDK 构建 Application、Ability、Manifest、Package、权限、服务依赖、GenUI surface、headless entry、plugin dependency 和 certification fixture。
- Application Service 暴露 sanitized metadata view，Web/CLI 不再读取 raw manifest 来决定系统语义。
- WASM Component ABI/WIT、guest SDK scaffold 和 unavailable-safe host 进入正式平台路线。
- GenUI、Headless、Store、Plugin-enhanced app 以 fixtures 和 certification tests 证明平台不是只为聊天/YAML demo 设计。

本计划只定义实施路线。后续必须先产出 OpenSpec proposal / design / tasks / spec，得到批准后再实施。

## 2. 必须遵守的治理约束

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/design_patterns.md`
- `docs/openharmony-microkernel-architecture-for-macaca-agent-os.md`
- `docs/superpowers/plans/2026-05-11-application-ecosystem-sdk-brainstorm.md`

硬性规则：

- Kernel 只承载 identity、scheduler、capability/service registry、policy facade、trace/audit、resource/session/task/package guard 等系统不变量。
- Application Framework 只拥有 application/ability/package/manifest/lifecycle/admission/compatibility 语义，不拥有 LLM、Memory、Driver、Skill、MCP、Plugin、Payment、Web3/EVM provider 实现。
- SDK 只能依赖 provider-neutral contracts，不得构造 `AppRuntime`、`Kernel`、`ServiceRuntime`、Web state 或 runtime-host provider。
- Web/CLI/Gateway 只能作为 shell/adapter，不得定义 application workflow、permission、runtime kind、payment、service dependency 的核心语义。
- YAML app 必须保持一等支持，但不允许继续成为新能力的特权中心。
- 禁止硬编码 app name、workflow name、driver name、gateway name、provider name、business name。
- 所有 service/application/ability/capability 操作必须带 trace 或返回 fail-closed/unavailable。
- 所有日志和 snapshots 必须脱敏，不得泄露 prompt body、raw manifest body、raw agent config、secret、env、API key、raw host payload、private key、unbounded user input。
- 所有新增 Rust 代码必须有详尽英文注释，解释功能、运行原理、边界和不变量。
- 每个 Rust 文件不得超过 500 行；接近上限时必须拆分模块。

## 3. 总体架构

```text
Developer / App Author
  ↓
Macaca SDK
  - ApplicationKit
  - AbilityKit
  - ManifestBuilder
  - PackageBuilder
  - HostServiceProxy
  - TestKit / CertificationKit
  ↓
Application Package Manifest v1
  - metadata / runtime / abilities / permissions / services
  - ui / commerce / plugin dependencies / compatibility
  ↓
Application Framework (`macaca-app`)
  - manifest loader
  - ability descriptor registry
  - YAML adapter
  - admission specifications
  - compatibility checker
  - sanitized metadata projector
  ↓
Application Service (`macaca-runtime-host` provider wrapper)
  - lifecycle commands
  - sanitized metadata query
  - host dispatch
  - unavailable-safe WASM/headless host skeleton
  ↓
System Services
  - Task / LLM / Memory / Context / Driver / Skill / MCP
  - Plugin / Store / Entitlement / Payment / Web3 / EVM / GenUI
```

设计模式必须贯穿：

- Facade：SDK kits、Application Service、SystemApplicationClient。
- Adapter：YAML legacy manifest、WASM metadata-only package、legacy `AppManifest`、future store package 都适配到 Manifest v1。
- Bridge：Application ABI / host imports 与 System Service Runtime 解耦。
- Abstract Factory：按 runtime kind / ability implementation 创建 host 或 unavailable-safe skeleton。
- Builder：manifest、ability、permission、service requirement、package、fixture 构造。
- Command：activation、host import、UI event、service call、certification check。
- Composite：Application 包含多个 Ability，Ability 包含 capability/permission/service declarations。
- Specification：manifest、ability、permission、ABI、compatibility、entitlement、trace-required admission。
- Observer：trace/audit/event stream。
- State：Application lifecycle 和 Ability lifecycle。
- Memento：snapshot、checkpoint、compatibility report、certification report。
- Null Object：缺失 optional service/runtime/module 时返回 structured unavailable。
- Visitor：compatibility/certification checker 遍历 manifest、ability、service、permission、package。

## 4. OpenSpec 拆分建议

建议一次性产出 6 个 OpenSpec 提案，按依赖顺序分阶段批准和实施：

1. `add-application-manifest-v1-ability-baseline`
2. `add-application-sdk-kits-v1`
3. `migrate-yaml-apps-to-manifest-v1-agent-ability`
4. `add-application-sanitized-metadata-service`
5. `add-wasm-component-application-abi-skeleton`
6. `add-application-platform-certification-fixtures`

如果希望减少变更目录数量，也可以用一个 umbrella proposal 加 6 个 spec delta，但不建议。该能力跨度大，分提案更利于审查、回滚和阶段性验证。

## 5. 阶段 1：Application Manifest v1 与 Ability Descriptor 基线

### 5.1 目标

定义 Application Platform 的事实来源：`ApplicationManifestV1` 和 `ApplicationAbilityDescriptor`。它们必须 provider-neutral、language-neutral、renderer-neutral，并能表达 YAML、WASM、GenUI、headless、hybrid、Store、Plugin-enhanced app。

### 5.2 主要范围

候选文件：

- `macaca/crates/foundation/macaca-proto/src/application_manifest.rs`
- `macaca/crates/foundation/macaca-proto/src/application_ability.rs`
- `macaca/crates/foundation/macaca-proto/src/application_abi.rs`
- `macaca/crates/foundation/macaca-proto/src/package.rs`
- `macaca/crates/application/macaca-app/src/manifest_v1/`
- `macaca/crates/application/macaca-app/src/ability/`
- `macaca/crates/application/macaca-app/src/compatibility_checker/`
- `macaca/crates/application/macaca-app/src/lib.rs`

新增或完善类型：

- `ApplicationManifestV1`
- `ApplicationManifestVersion`
- `ApplicationRuntimeProfile`
- `ApplicationRuntimeKind`
- `ApplicationAbilityDescriptor`
- `ApplicationAbilityKind`
- `AbilityImplementationKind`
- `AbilityActivation`
- `AbilityLifecyclePolicy`
- `AbilityPermissionDeclaration`
- `AbilityServiceRequirement`
- `AbilityCapabilityDeclaration`
- `AbilityUiSurfaceDeclaration`
- `ApplicationCommerceDeclaration`
- `ApplicationPluginDependency`
- `ApplicationCompatibilityDeclaration`

最小 Ability 集合：

- `AgentAbility`
- `UiAbility`
- `HeadlessAbility`
- `ScheduledAbility`
- `GatewayAbility`
- `ExtensionAbility`

### 5.3 设计要求

- `ApplicationManifestV1` 是 package/application 事实来源，不能直接复用 legacy YAML `AppManifest` 作为新事实来源。
- legacy `AppManifest` 通过 Adapter 转换，不删除、不改语义。
- Ability descriptor 只描述能力边界，不执行 provider。
- permission/service/capability declaration 必须可被 Specification 校验。
- runtime kind 选择必须基于 manifest/ABI，不得基于 app name 或目录名。
- 类型归属优先放 `macaca-proto`，让 SDK、app framework、runtime-host、Web/CLI 都共享同一 contract。

### 5.4 验证

- 单元测试：manifest v1 序列化/反序列化。
- 单元测试：ability descriptor 支持所有最小 ability kind。
- 单元测试：permission/service/capability declaration 去重、排序、稳定输出。
- 集成测试：Route C dependency boundaries 不新增违规依赖。
- `cargo test -p macaca-proto application_manifest`
- `cargo test -p macaca-app manifest_v1`
- `cargo test -p macaca-integration-tests route_c_dependency_boundaries`

## 6. 阶段 2：SDK ApplicationKit / AbilityKit / Manifest Builder / TestKit

### 6.1 目标

让第三方开发者不需要手写底层 DTO 或理解内部 crate，就能通过 SDK 声明 application、ability、permissions、services、GenUI surface、headless entry 和 certification fixture。

### 6.2 主要范围

候选文件：

- `macaca/crates/facade/macaca-sdk/src/application.rs`
- `macaca/crates/facade/macaca-sdk/src/application_kit/`
- `macaca/crates/facade/macaca-sdk/src/ability_kit/`
- `macaca/crates/facade/macaca-sdk/src/application_testkit/`
- `macaca/crates/facade/macaca-sdk/src/package_fixtures.rs`
- `macaca/crates/facade/macaca-sdk/examples/`
- `macaca/crates/facade/macaca-sdk/src/lib.rs`

新增 SDK kits：

- `ApplicationKit`
- `AbilityKit`
- `ApplicationManifestBuilder`
- `AbilityDescriptorBuilder`
- `ApplicationPackageBuilder`
- `ApplicationPermissionBuilder`
- `ApplicationServiceRequirementBuilder`
- `ApplicationCapabilityBuilder`
- `GenUiSurfaceBuilder`
- `ApplicationContractTestKit`
- `ApplicationFixtureBuilder`

### 6.3 设计要求

- SDK builder 必须依赖 `macaca-proto` contract，不依赖 `macaca-app` runtime concrete。
- SDK 提供高层 builder，但最终产物必须是可序列化、可审计、provider-neutral DTO。
- TestKit 必须验证 manifest required fields、ability required fields、trace-required import、permission/service consistency、forbidden hardcoded runtime assumptions。
- SDK examples 必须覆盖 YAML-equivalent declarative app、GenUI app、headless app、plugin-enhanced app、WASM skeleton app。
- `SystemApplicationClient` 继续保留 shell-facing 职责，不与 Developer SDK 混成一个巨型 facade。

### 6.4 验证

- 单元测试：SDK builder 输出 deterministic manifest。
- 单元测试：TestKit 能拒绝 missing permission / missing service / missing ability entry。
- 示例测试：examples 可编译或作为 fixture 通过 contract tests。
- `cargo test -p macaca-sdk application_kit`
- `cargo test -p macaca-sdk application_testkit`
- `cargo check -p macaca-sdk`

## 7. 阶段 3：YAML app adapter 迁移到 Manifest v1 + AgentAbility

### 7.1 目标

把现有 YAML application 从“旧事实来源”迁移为 “Manifest v1 + AgentAbility 的兼容 profile”。行为不变，语义归属改变。

### 7.2 主要范围

候选文件：

- `macaca/crates/application/macaca-app/src/model.rs`
- `macaca/crates/application/macaca-app/src/loader.rs`
- `macaca/crates/application/macaca-app/src/package.rs`
- `macaca/crates/application/macaca-app/src/abi.rs`
- `macaca/crates/application/macaca-app/src/runtime.rs`
- `macaca/crates/application/macaca-app/src/manifest_v1/yaml_adapter.rs`
- `macaca/crates/tests/macaca-integration-tests/tests/app_declarative.rs`

新增或完善：

- `YamlApplicationManifestAdapter`
- `LegacyAppManifestProjection`
- `AgentAbilityFromYamlSpec`
- `YamlToApplicationManifestV1Report`
- deprecated anchors for direct legacy manifest startup/read paths where needed。

### 7.3 设计要求

- 当前 YAML app 加载、entry agent 解析、agent config resolution、workflow/resource/context 字段语义不能退化。
- YAML 转换为 `AgentAbility`，不直接把所有字段塞进顶层 manifest。
- 转换 report 必须指出 legacy-only 字段、默认推导、兼容性 warnings。
- AppPackageDescriptorBuilder 应优先从 Manifest v1 生成 package descriptor；旧路径保留 deprecated anchor。
- 不删除旧 `AppManifest`，但新生产路径应以 Manifest v1 projection 为中心。

### 7.4 验证

- 现有 YAML app integration tests 继续通过。
- 新增测试：同一个 YAML app 转 Manifest v1 后 package descriptor/ABI descriptor 与旧路径关键字段一致。
- 新增测试：YAML app 生成至少一个 `AgentAbility`。
- `cargo test -p macaca-app yaml`
- `cargo test -p macaca-integration-tests --test app_declarative`
- `cargo check --workspace`

## 8. 阶段 4：Application Service sanitized metadata view 替代 Web raw manifest reads

### 8.1 目标

Application Service 提供 sanitized metadata view，让 Web/CLI/Gateway 获取 app/ability/runtime/capability/entry/session 所需信息时不再直接读取 raw manifest 或复制 Application Framework 语义。

### 8.2 主要范围

候选文件：

- `macaca/crates/foundation/macaca-proto/src/application_service.rs`
- `macaca/crates/application/macaca-app/src/service_adapter.rs`
- `macaca/crates/application/macaca-app/src/service_projection.rs`
- `macaca/crates/runtime/macaca-runtime-host/src/application_service_provider.rs`
- `macaca/crates/facade/macaca-sdk/src/application_client.rs`
- `macaca/crates/shells/macaca-web/src/routes.rs`
- `macaca/crates/shells/macaca-web/src/chat_orchestrator.rs`
- `macaca/crates/shells/macaca-web/src/framework_runner.rs`
- `macaca/crates/shells/macaca-web/src/framework_toolkit.rs`
- `macaca/crates/shells/macaca-web/src/loop_manager.rs`
- `macaca/crates/shells/macaca-web/src/skill_mcp.rs`

新增或完善 DTO：

- `ApplicationMetadataQueryCommand`
- `ApplicationMetadataView`
- `ApplicationAbilityView`
- `ApplicationEntryView`
- `ApplicationToolPolicyView`
- `ApplicationContextPolicyView`
- `ApplicationSkillPolicyView`
- `ApplicationMcpOverlayView`
- `ApplicationSanitizedManifestDigest`

### 8.3 设计要求

- Metadata view 必须只暴露 shell/framework 执行所需的 sanitized projection，不暴露 raw prompt、raw full manifest、secret、env、credential。
- Web 读取 entry agent、agent list、tool policy、context policy、skill/MCP overlay 时必须优先走 `SystemApplicationClient`。
- direct raw manifest reads 保留 deprecated fallback，并标注过期条件。
- 迁移不得改变 `/api/chat/v2`、session trace、goal resume、toolkit、skill/MCP overlay 现有行为。
- Application Service 不得吸收 Task/LLM/Driver/Skill/MCP 执行语义，只提供 app-owned metadata view。

### 8.4 GitNexus 与影响控制

修改 Web 现有函数前必须运行 GitNexus impact：

- `post_chat`
- `FrameworkRunner::run`
- `FrameworkRunner::build_system_prompt`
- `build_framework_toolkit`
- `get_app_skills`
- `resolve_session_application`
- 任何直接读取 `state.registry`、`runtime.registry`、`AppManifest` 的函数。

如果 GitNexus 返回 HIGH/CRITICAL，先报告 blast radius，再拆更小迁移切片。

### 8.5 验证

- `rg` 证明新增代码不再引入新的 raw manifest reads。
- 旧 raw manifest reads 只能出现在 deprecated fallback 或 compatibility tests。
- `cargo test -p macaca-integration-tests route_c_dependency_boundaries`
- `cargo test -p macaca-integration-tests route_c_workspace_topology`
- `cargo test -p macaca-web` 或可用的 Web crate tests。
- `cargo check --workspace`

## 9. 阶段 5：WASM Component ABI/WIT skeleton + guest SDK scaffold + unavailable-safe host

### 9.1 目标

把 WASM Component Application 从“报告里的愿景”推进为正式 ABI skeleton：有 WIT/schema、有 guest SDK scaffold、有 host factory、有 unavailable-safe execution result。此阶段仍不要求真实执行第三方 WASM。

### 9.2 主要范围

候选文件：

- `macaca/crates/foundation/macaca-proto/src/application_abi.rs`
- `macaca/crates/application/macaca-app/src/abi.rs`
- `macaca/crates/application/macaca-app/src/wasm/`
- `macaca/crates/runtime/macaca-runtime-host/src/application_hosts/`
- `macaca/crates/facade/macaca-sdk/src/application_kit/wasm.rs`
- `macaca/application-wit/` 或 `macaca/resources/application-wit/`
- `macaca/crates/facade/macaca-sdk/examples/wasm_component_app_fixture.rs`

新增或完善：

- `macaca-application.wit` 或等价 ABI schema。
- `WasmComponentApplicationDescriptor`
- `WasmGuestImport`
- `WasmGuestExport`
- `WasmApplicationHostFactory`
- `UnavailableWasmApplicationHost`
- guest SDK scaffold docs/examples。

### 9.3 设计要求

- WIT/ABI schema 必须与 `ApplicationImport` / `ApplicationExport` 对齐。
- Host imports 必须通过 Command/Bridge 进入 service call，不得直接绑定 provider。
- 缺失 WASM runtime 时必须返回 structured unavailable，包含 trace、runtime kind、reason code。
- WASM package admission 可通过 manifest/ABI/permissions/service dependency validation，但不得假装执行成功。
- 不引入重量级 WASM runtime 依赖，除非后续 OpenSpec 明确批准。

### 9.4 验证

- ABI schema 文件存在并被 tests 校验与 Rust DTO 对齐。
- WASM skeleton app fixture 能通过 manifest/ABI/SDK TestKit。
- Host dispatch to WASM skeleton returns structured unavailable。
- `cargo test -p macaca-proto application_abi`
- `cargo test -p macaca-app wasm`
- `cargo test -p macaca-runtime-host application_hosts`
- `cargo test -p macaca-integration-tests --test app_declarative` 或新增 application platform test。

## 10. 阶段 6：GenUI / Headless / Store / Plugin-enhanced app fixtures 与 certification tests

### 10.1 目标

通过 fixtures 和 certification tests 证明 Application Platform 能支撑真实生态形态，而不是只支持 YAML chat demo。

### 10.2 主要范围

候选文件：

- `macaca/crates/tests/macaca-integration-tests/tests/application_platform_contracts.rs`
- `macaca/crates/tests/macaca-integration-tests/tests/application_platform_contracts/fixtures.rs`
- `macaca/crates/facade/macaca-sdk/examples/genui_app_fixture.rs`
- `macaca/crates/facade/macaca-sdk/examples/headless_app_fixture.rs`
- `macaca/crates/facade/macaca-sdk/examples/plugin_enhanced_app_fixture.rs`
- `macaca/crates/facade/macaca-sdk/examples/store_entitled_app_fixture.rs`
- `macaca/crates/application/macaca-app/src/certification/`

Certification coverage：

- Declarative YAML app maps to AgentAbility。
- GenUI app declares UI surface, event schema, permission, trace policy。
- Headless app declares activation and service dependency without UI。
- Store-entitled app declares commerce metadata and entitlement dependency。
- Plugin-enhanced app declares plugin dependency and capability import。
- WASM skeleton app declares ABI and returns unavailable-safe execution。
- Missing permission/service/plugin/runtime is rejected or unavailable with structured diagnostics。
- Sanitized metadata view never leaks raw prompt, secret, env, raw manifest body。

### 10.3 设计要求

- Fixtures 必须是 generic，不硬编码业务 app 名称。
- CertificationKit 使用 Visitor 遍历 manifest/ability/service/permission/plugin/commerce/ABI。
- Tests 不应依赖真实 network、真实 store、真实 payment、真实 WASM runtime、真实 Web3/EVM。
- Certification report 必须可序列化，可作为未来 Store submission checklist 的基础。

### 10.4 验证

- `cargo test -p macaca-integration-tests --test application_platform_contracts`
- `cargo test -p macaca-app certification`
- `cargo test -p macaca-sdk application_testkit`
- `cargo check --workspace`
- `npx gitnexus detect-changes -r agent`

## 11. 实施顺序与依赖

必须按顺序实施：

1. Manifest v1 / Ability Descriptor。
2. SDK Kits。
3. YAML Adapter migration。
4. Sanitized metadata service view。
5. WASM ABI skeleton。
6. Certification fixtures。

依赖关系：

- 阶段 2 依赖阶段 1 的 DTO。
- 阶段 3 依赖阶段 1 的 manifest/ability model。
- 阶段 4 依赖阶段 1 和 3 的 projection。
- 阶段 5 依赖阶段 1 的 runtime/ABI declaration，并可复用阶段 2 TestKit。
- 阶段 6 依赖全部前序阶段。

## 12. 回滚策略

- 所有阶段 additive-first，不删除 legacy YAML AppManifest、AppRuntime、AppLoader、Web fallback。
- 旧接口只标记 deprecated 和禁止新生产调用，保留搜索锚点。
- 每个阶段完成后单独提交。
- 如果某阶段失败，可保留 DTO/SDK skeleton，但不迁移消费者。
- Web raw manifest reads 迁移必须有 fallback，直到行为回归测试稳定。

## 13. 完成标准

整体完成后应满足：

- `ApplicationManifestV1` 和 Ability Descriptor 成为新 app 能力事实来源。
- SDK 提供 ApplicationKit / AbilityKit / Manifest Builder / TestKit。
- YAML app 通过 Adapter 映射为 Manifest v1 + AgentAbility，旧行为不退化。
- Application Service 提供 sanitized metadata view，Web 新路径不再直接依赖 raw manifest 做核心语义判断。
- WASM Component ABI/WIT skeleton 和 guest SDK scaffold 存在，host unavailable 行为可测试。
- GenUI、Headless、Store、Plugin-enhanced、WASM skeleton fixtures 通过 certification tests。
- Route C dependency/topology gates 通过。
- `cargo check --workspace` 通过。
- GitNexus detect-changes 范围符合预期。

## 14. 下一步

先产出第一个 OpenSpec 提案：

- `add-application-manifest-v1-ability-baseline`

该提案只实现阶段 1，不迁移 Web，不实现 WASM runtime，不改 YAML 行为。阶段 1 完成并验证后，再进入 SDK Kits 提案。
