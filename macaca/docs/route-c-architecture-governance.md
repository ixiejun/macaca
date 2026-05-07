# Macaca OS 路线 C 架构治理规则

## 1. 治理目标

路线 C 的目标是让 Macaca 成为微内核 Agent OS，而不是宏内核 Agent 平台。治理规则用于阻止后续实现中出现短期可跑、长期不可维护的捷径。

## 2. 架构层级规则

### 2.1 Kernel Rule

Kernel 只负责系统不变量。新增能力进入 kernel 前必须证明：

- 所有 application 都无法绕开该能力。
- 该能力不依赖具体 provider。
- 该能力不包含业务 workflow。
- 该能力不会因为第三方生态扩展频繁变化。

### 2.2 Service Rule

如果能力可以被替换 provider、第三方实现或远程实现替换，它必须是 system service。

### 2.3 Plugin Rule

如果能力由第三方扩展系统表面，它必须通过 plugin manifest、service registry、permission、trace、lifecycle 接入。

### 2.4 Optional Module Rule

Web3、EVM、特定 gateway、特定 driver、特定 paid package 都必须是可选模块。缺失时返回结构化 unavailable，不影响 base OS。

### 2.5 Presentation Rule

Web/CLI/Frontend 只能是 shell 和 adapter，不得定义核心 session、task、trace、payment、package 语义。

## 3. 设计模式规则

优先使用：

- Facade：隐藏底层系统复杂度。
- Adapter / Bridge：隔离外部 provider、driver、gateway、MCP、Web3。
- Strategy：调度、policy、provider、计费策略可替换。
- Command：service call、UI event、payment intent、contract call。
- Observer：trace/audit/event stream。
- State：session、task、plugin、payment lifecycle。
- Memento：session checkpoint、task history、receipt、event replay。
- Specification：manifest、permission、compatibility、entitlement validation。

禁止用大段 if/else 判断 app/provider/driver/gateway 名称来替代可扩展模式。

## 4. 可观测性规则

以下动作必须有 trace：

- app lifecycle。
- agent lifecycle。
- task lifecycle。
- service call。
- driver call。
- skill/MCP call。
- plugin lifecycle。
- entitlement check。
- payment intent。
- Web3 transaction。
- EVM contract call。
- UI event。

无 trace，不执行。

## 5. 权限规则

所有 capability call 必须经过 policy：

- application permissions。
- plugin permissions。
- user approvals。
- spending budgets。
- region compliance。
- optional module availability。

无权限，不调用。

## 6. 兼容规则

- 现有 YAML application 在正式迁移路径出现前必须保持一等支持。
- 现有 `/api/chat/v2`、trace、task board、resume 链路不得退化。
- 新 package/ABI/Store/Web3 能力必须 additive-first。

## 7. 审查清单

任何 Route C OpenSpec 都必须回答：

- 这个能力属于 kernel、service、plugin、optional module、application framework 还是 presentation？
- 是否存在 provider/app/driver/gateway hardcode？
- 是否支持 trace？
- 是否支持 permission/policy？
- 缺失 optional module 时如何表现？
- 如何验证不破坏 Route C regression matrix？

