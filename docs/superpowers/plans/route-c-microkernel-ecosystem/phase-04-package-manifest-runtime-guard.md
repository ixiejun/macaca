# 阶段 4：Package Manifest 与 Runtime Guard 细分实施计划

## 目标

建立 Macaca 统一 package metadata 与 runtime guard。现有 YAML application 必须被表示为 package 的一种兼容形态，未来 WASM app、plugin、skill、MCP、driver、system module 都使用同一套 package contract。

## 架构设计

Package 是 Macaca OS 软件生态的最小分发单元。Manifest 描述“这个包是什么、需要什么、提供什么、如何运行、如何授权、如何验证”。Runtime Guard 负责在加载前阻止不兼容、不授权、不安全的包。

推荐设计模式：

- Builder：从 manifest 构建 `PackageDescriptor`。
- Specification：兼容性、权限、签名、runtime kind 都是可组合规则。
- Chain of Responsibility：manifest validation -> signature validation -> compatibility validation -> permission validation -> entitlement precheck。
- Factory Method：按 package type 创建 loader。
- Null Object：未安装 optional module 时返回 unavailable service，不 panic。

## 涉及文件

- 新增：`macaca/crates/macaca-proto/src/package.rs`
- 修改：`macaca/crates/macaca-proto/src/lib.rs`
- 新增：`macaca/crates/macaca-app/src/package.rs`
- 新增：`macaca/crates/macaca-app/src/package_loader.rs`
- 新增：`macaca/crates/macaca-app/src/runtime_guard.rs`
- 新增：`macaca/crates/macaca-app/tests/package_manifest.rs`
- 修改：`macaca/crates/macaca-skill`
- 修改：`macaca/crates/macaca-driver`
- 修改：`macaca/crates/macaca-runtime-host`

## 抽象设计

Manifest v0 必须支持：

- `package_id`
- `package_type`
- `version`
- `developer_id`
- `signature`
- `runtime.kind`
- `runtime.abi_version`
- `entry`
- `permissions`
- `requires.services`
- `requires.optional_services`
- `provides.capabilities`
- `commerce.license`
- `compatibility.min_os_version`

Package type：

- application
- skill
- plugin
- mcp
- driver
- system_module
- ui_component_pack

Runtime kind：

- yaml
- wasm_component
- native_adapter
- remote_service
- encrypted_text_bundle

## 实施切片

### 切片 4.1：proto package schema

在 `macaca-proto` 定义 package manifest 数据结构，必须 serde roundtrip。

验证：

- 每种 package type 都有 fixture。
- unknown future type 不导致解析崩溃。

### 切片 4.2：YAML application compatibility adapter

把当前 app.yaml/persona/tools/workflow 映射为 `PackageDescriptor`。

验证：

- FULLSTACK-AUTODEV 和 NEWSROOM-AUTOWRITER 都能生成 package descriptor。
- 生成结果包含 app id、entry agent、required services、allowed tools。

### 切片 4.3：runtime guard validation chain

实现 guard chain：

```text
parse -> schema -> compatibility -> permission -> optional service availability -> commerce inert check
```

验证：

- 缺 runtime kind 被拒绝。
- 不兼容 ABI 被拒绝。
- 缺 required service 被拒绝。
- 缺 optional service 不拒绝，但标记 unavailable。

### 切片 4.4：package loader factory

按 runtime kind 选择 loader。第一版必须真实支持 YAML loader；WASM loader 只允许加载 metadata，不执行代码。

验证：

- YAML loader 可加载现有应用。
- WASM component package 如果无执行 runtime，返回明确 `RuntimeUnavailable`。

## 里程碑

- M4.1：Manifest v0 类型稳定。
- M4.2：现有 YAML app 能映射 package。
- M4.3：Runtime Guard 可拒绝非法 package。
- M4.4：Loader factory 可扩展。

## 禁止事项

- 禁止把 commerce metadata 当成真正支付实现。
- 禁止为了通过测试跳过 runtime guard。
- 禁止把 YAML application 降级为二等 legacy。
- 禁止在 manifest schema 中硬编码某个 demo app。

## 验收命令

```bash
cargo test -p macaca-proto package
cargo test -p macaca-app package_manifest
cargo check -p macaca-web
```

