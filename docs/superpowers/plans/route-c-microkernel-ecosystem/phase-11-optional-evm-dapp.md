# 阶段 11：可选 EVM / DApp Module 细分实施计划

## 目标

在可选 Web3 模块基础上，定义 EVM/DApp 能力，使开发者可以构建 AI + Web3 application。Macaca 不自研底层链和 EVM，而是基于 Substrate/Frontier 等成熟方案做 adapter。

## 架构设计

EVM 是 Web3 的 optional submodule。Application 通过 capability 声明需要 `web3.evm`，系统根据模块是否安装决定 allow/unavailable。Contract call 必须是 service command，必须 trace，必须通过 signing/payment policy。

推荐设计模式：

- Adapter：Substrate/Frontier/EVM RPC 适配为 Macaca EVM service。
- Command：contract deploy/call/read/subscribe 都是命令。
- Strategy：gas policy、network selection、signing policy 可替换。
- Observer：contract event subscription 转为 trace/event stream。
- Facade：DApp 通过 SDK facade 调用 EVM。

## 涉及文件

- 新增：`macaca/crates/macaca-proto/src/evm.rs`
- 未来新增：`macaca/crates/macaca-evm`
- 修改：`macaca/crates/macaca-app`
- 修改：`macaca/crates/macaca-sdk`
- 修改：`macaca/crates/macaca-web3`
- 新增测试：`macaca/crates/macaca-evm/tests/evm_contract.rs`

## 抽象设计

EVM service contract：

- `deploy_contract`
- `call_contract`
- `read_contract_state`
- `subscribe_contract_events`
- `estimate_gas`
- `get_transaction_receipt`

核心类型：

- `EvmChainId`
- `ContractAddress`
- `ContractAbiRef`
- `ContractCallRequest`
- `ContractCallResult`
- `GasPolicy`
- `ContractEvent`

## 实施切片

### 切片 11.1：EVM proto

定义 EVM 类型与 request/result。

验证：

- contract call fixture roundtrip。
- gas policy 不写死具体链。

### 切片 11.2：EVM unavailable behavior

未安装 EVM module 时，所有 EVM capability 返回 unavailable。

验证：

- 普通 app 不受影响。
- DApp 获取结构化 unavailable error。

### 切片 11.3：mock EVM adapter

实现 mock adapter 验证 deploy/call/read/receipt 语义。

验证：

- mock deploy 返回 contract address。
- mock call 返回 transaction receipt。
- 所有命令 emit trace。

### 切片 11.4：Substrate/EVM adapter design doc

写清未来真实接入 Substrate/Frontier 的边界，不在本阶段运行真实节点。

验证：

- 文档明确哪些在 adapter，哪些在 kernel，哪些在 optional module。

## 里程碑

- M11.1：EVM 类型稳定。
- M11.2：EVM absent-safe。
- M11.3：Mock EVM adapter 跑通。
- M11.4：Substrate/EVM adapter 边界明确。

## 禁止事项

- 禁止实现自研 EVM。
- 禁止 EVM 进入 base OS 必装路径。
- 禁止 contract call 绕过 signing/payment policy。
- 禁止把测试 mock 冒充真实链能力。

## 验收命令

```bash
cargo test -p macaca-proto evm
cargo test -p macaca-evm evm_contract
cargo check --workspace
```

