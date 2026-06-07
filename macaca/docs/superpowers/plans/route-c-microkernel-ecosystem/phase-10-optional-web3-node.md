# 阶段 10：可选 Web3 Node Module 细分实施计划

## 目标

把 Web3 node 能力做成可选安装模块。未安装时 base OS 必须完整可用；安装后提供 wallet、signing、transaction、chain query、agent payment adapter 等能力。

## 架构设计

Web3 是 optional system module，不是基础内核能力。Kernel 只知道是否有 `web3.wallet`、`web3.signing`、`web3.transaction` 等 service，不能知道具体链实现。

推荐设计模式：

- Null Object：未安装模块时返回 `UnavailableWeb3Service`。
- Adapter：不同链、不同钱包、不同节点实现适配为统一 Web3 service。
- Proxy：节点可以本地运行，也可以远程 RPC。
- Strategy：signing policy、fee policy、network policy 可替换。
- Facade：Application 只通过 Web3 facade 调用，不接触私钥。

## 涉及文件

- 新增：`macaca/crates/macaca-proto/src/web3.rs`
- 未来新增：`macaca/crates/macaca-web3`
- 修改：`macaca/crates/macaca-kernel/src/service_registry.rs`
- 修改：`macaca/crates/macaca-ipc`
- 修改：`macaca/crates/macaca-app`
- 修改：`macaca/crates/macaca-web`

## 抽象设计

Web3 service types：

- `web3.wallet`
- `web3.signing`
- `web3.transaction`
- `web3.chain_query`
- `web3.payment_adapter`

核心类型：

- `WalletId`
- `ChainId`
- `Address`
- `SigningRequest`
- `SigningPolicy`
- `TransactionRequest`
- `TransactionReceipt`
- `Web3Availability`

## 实施切片

### 切片 10.1：Web3 proto 和 unavailable service

定义 Web3 类型和 unavailable service 行为。

验证：

- 未安装 Web3 时查询 availability 返回 unavailable。
- Application 请求 Web3 capability 得到结构化错误。

### 切片 10.2：wallet/signing service contract

定义 wallet/signing trait，不实现真实私钥管理。

验证：

- mock wallet 可产生 signing request。
- signing request 必须经过 policy。

### 切片 10.3：transaction/chain query contract

定义 transaction 和 chain query command。

验证：

- mock transaction 返回 receipt。
- chain query 不可用时不影响 base OS。

### 切片 10.4：合规与地区禁用状态

Web3 module 支持 region/compliance disabled。

验证：

- disabled region 下所有 Web3 calls 返回 policy denied。
- deny event 进入 trace。

## 里程碑

- M10.1：Web3 absent-safe。
- M10.2：Wallet/signing contract 可用。
- M10.3：Transaction/chain query contract 可用。
- M10.4：Compliance policy 生效。

## 禁止事项

- 禁止默认安装或默认启用 Web3。
- 禁止 application 读取私钥。
- 禁止把某条链写死到 kernel。
- 禁止 Web3 缺失导致普通 app 失败。

## 验收命令

```bash
cargo test -p macaca-proto web3
cargo test -p macaca-kernel web3
cargo check --workspace
```

