# 阶段 8：Store / Entitlement v0 细分实施计划

## 目标

建立 Macaca OS 商业生态基础：Store package source、签名验证、license、订阅、用量计费、encrypted skill loading hook、paid application install guard。阶段 8 必须让付费能力可表达、可拒绝、可追踪，但不要求接入真实支付渠道。

## 架构设计

Store/Entitlement 是 OS 生态基础设施，不是某个 Web 页面。所有 paid skill、paid application、paid plugin、paid MCP 都必须通过 entitlement service 决定是否可安装、可启动、可调用。

推荐设计模式：

- Facade：`EntitlementService` 对外提供统一授权判断。
- Strategy：license 类型和计费策略可替换。
- Proxy：高价值付费能力未来可走 remote execution proxy。
- Chain of Responsibility：signature -> entitlement -> metering -> runtime guard。
- Decorator：在 package/service call 外层追加 entitlement/metering。

## 涉及文件

- 新增：`macaca/crates/macaca-proto/src/commerce.rs`
- 新增：`macaca/crates/macaca-persist/src/entitlement_store.rs`
- 新增：`macaca/crates/macaca-runtime-host/src/entitlement.rs`
- 新增：`macaca/crates/macaca-skill/src/encrypted_package.rs`
- 新增：`macaca/crates/macaca-app/src/commercial_package.rs`
- 未来新增：`macaca/crates/macaca-store`
- 修改：`macaca/crates/macaca-web`
- 新增测试：`macaca/crates/macaca-runtime-host/tests/entitlement.rs`

## 抽象设计

Commerce metadata：

- `license_type`
- `store_required`
- `developer_id`
- `package_signature`
- `entitlement_id`
- `subscription_plan`
- `metering_events`
- `offline_grace_period`
- `revocation_policy`

Entitlement states：

- valid
- expired
- missing
- revoked
- region_blocked
- usage_exceeded
- unknown_offline

## 实施切片

### 切片 8.1：commerce proto

定义 commerce、license、entitlement、metering 类型。

验证：

- free/open/paid/subscription/metered fixture 都能 roundtrip。
- unknown license type 不导致 panic。

### 切片 8.2：entitlement store

新增 entitlement persistence contract。

验证：

- 可写入 entitlement。
- 可查询 entitlement。
- revoked entitlement 会覆盖 valid 状态。

### 切片 8.3：runtime entitlement guard

package/runtime guard 调用 entitlement service。免费/开源允许；付费无 entitlement 拒绝。

验证：

- free package 可运行。
- paid package 无 entitlement 被拒绝。
- paid package 有 valid entitlement 可运行。

### 切片 8.4：encrypted skill hook

定义 encrypted skill loading hook。第一版只要求加密包识别、授权检查、解密接口抽象，不做弱假加密冒充真实保护。

验证：

- encrypted skill 无 entitlement 拒绝加载。
- 有 entitlement 进入 decrypt hook。
- decrypt hook 失败返回结构化错误。

### 切片 8.5：metering event

调用 paid capability 时产生 metering event。

验证：

- metering event 进入 EventLog。
- event 包含 app/package/developer/session/capability。

## 里程碑

- M8.1：commerce metadata 可表达。
- M8.2：entitlement store 可用。
- M8.3：paid package 可被 allow/deny。
- M8.4：encrypted skill loading hook 可接入。
- M8.5：metering trace 可见。

## 禁止事项

- 禁止假装本地加密能绝对防破解。
- 禁止 paid package 绕过 entitlement guard。
- 禁止 Store 阻断本地开发和开源包。
- 禁止在本阶段接入真实支付 provider。

## 验收命令

```bash
cargo test -p macaca-proto commerce
cargo test -p macaca-persist entitlement
cargo test -p macaca-runtime-host entitlement
cargo test -p macaca-skill encrypted_package
```

