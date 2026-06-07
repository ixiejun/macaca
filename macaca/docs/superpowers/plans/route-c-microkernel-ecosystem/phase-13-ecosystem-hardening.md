# 阶段 13：生态硬化细分实施计划

## 目标

让 Macaca OS 具备真实第三方开发者生态的基础条件：开发文档、SDK 示例、package compatibility checker、签名/权限/entitlement 检查、certification tests、Store submission checklist、安全审计和升级兼容策略。

## 架构设计

生态硬化不是写市场宣传文档，而是让第三方开发者可以在不修改 Macaca 源码的情况下开发、打包、安装、运行、trace、调试、发布并可选商业化 application/plugin/skill/MCP。

推荐设计模式：

- Specification：兼容性、权限、包格式、认证规则都可检查。
- Builder：开发者工具从模板生成 package。
- Visitor：compatibility checker 遍历 manifest/ABI/dependencies。
- Facade：SDK 对开发者隐藏内部 crate。
- Template Method：certification tests 每类 package 使用统一测试流程。

## 涉及文件

- 新增：`macaca/docs/developer/application-development-guide.md`
- 新增：`macaca/docs/developer/plugin-development-guide.md`
- 新增：`macaca/docs/developer/genui-development-guide.md`
- 新增：`macaca/docs/developer/store-submission-guide.md`
- 新增：`macaca/docs/developer/web3-dapp-development-guide.md`
- 新增：`macaca/crates/macaca-sdk/examples`
- 新增：`macaca/crates/macaca-integration-tests/tests/package_certification.rs`
- 新增：`macaca/crates/macaca-app/src/compatibility_checker.rs`
- 修改：`macaca/docs/SYSTEM_OVERVIEW.md`

## 必须覆盖的开发者路径

- YAML app 开发、安装、运行。
- WASM-stub app 打包、加载、不可执行时的结构化错误。
- GenUI app 输出 UI schema、接收 UI event。
- Gateway plugin 注册外部入口。
- Driver plugin 注册 driver capability。
- Skill package 明文与 encrypted metadata。
- Paid package entitlement deny/allow。
- Web3 unavailable-safe app。
- EVM unavailable-safe DApp。

## 实施切片

### 切片 13.1：开发者文档体系

创建 developer docs 目录，按 application/plugin/genui/store/web3 分类。

验证：

- 每份文档都有最小真实示例。
- 每份文档说明权限、trace、package、调试方式。

### 切片 13.2：SDK examples

新增 examples：

- yaml-app-fixture
- wasm-stub-app-fixture
- genui-app-fixture
- gateway-plugin-fixture
- paid-skill-fixture
- web3-optional-fixture

验证：

- examples 可被 compatibility checker 读取。
- 不要求所有 examples 可执行真实外部服务，但必须通过 schema/guard。

### 切片 13.3：Compatibility checker

实现 checker，检查 manifest、ABI version、permissions、required services、optional modules、commerce metadata。

验证：

- 合法 package 通过。
- 缺权限、缺 ABI、缺 required service 的 package 被拒绝。

### 切片 13.4：Certification tests

为每类生态能力建立 certification test。

验证：

- `cargo test -p macaca-integration-tests package_certification` 能跑。
- 每个失败输出具体不兼容原因。

### 切片 13.5：升级兼容策略

定义 OS version、ABI version、package manifest version 的兼容规则。

验证：

- checker 能区分 compatible、warning、incompatible。

## 里程碑

- M13.1：开发者文档覆盖主要 package 类型。
- M13.2：SDK examples 可被检查。
- M13.3：Compatibility checker 可用。
- M13.4：Certification tests 可运行。
- M13.5：升级兼容策略明确。

## 禁止事项

- 禁止只写概念文档没有可检查规则。
- 禁止把 certification test 写成只检查文件存在。
- 禁止要求第三方开发者修改 Macaca 源码。
- 禁止忽略 paid/package/Web3 optional 场景。

## 验收命令

```bash
cargo test -p macaca-app compatibility_checker
cargo test -p macaca-integration-tests package_certification
rg -n "YAML|WASM|GenUI|Plugin|Store|Web3|EVM" macaca/docs/developer
```

