## ADDED Requirements

### Requirement: Macaca SHALL provide a pluggable context engine contract

Macaca SHALL 提供可插拔的上下文引擎契约，用于在模型调用前组装、诊断和后处理上下文。上层 runtime、framework、web 和 application 代码 MUST 依赖该契约或门面，而不是依赖具体上下文引擎实现。

#### Scenario: Default legacy engine preserves current behavior

- **GIVEN** 系统未显式配置自定义 context engine
- **WHEN** framework 或 runtime 发起一次模型请求
- **THEN** Macaca 必须使用默认 `legacy` engine
- **AND** 发送给 LLM provider 的 messages/options 必须与接入前保持行为兼容
- **AND** 请求必须产生基础 `ContextReport`

#### Scenario: Engine selection is configuration driven

- **GIVEN** application manifest、agent profile 或系统配置指定 context engine id
- **WHEN** Macaca 创建模型请求上下文
- **THEN** 系统必须通过注册表或 provider factory 选择对应 context engine
- **AND** core 代码不得通过硬编码 app name、workflow name、driver name 或业务名称选择 context engine

#### Scenario: Custom engine can replace default implementation

- **GIVEN** 用户提供了符合 `ContextEngine` 契约的自定义实现
- **WHEN** 该实现被注册并通过配置选中
- **THEN** Macaca 必须通过同一抽象调用该实现
- **AND** framework/runtime/web 上层代码不得依赖该自定义实现的具体类型

### Requirement: Context engine boundaries SHALL be extensible without strong coupling

Macaca SHALL 允许用户或内置模块在不同层级替换上下文行为，包括 engine、source provider、policy 和未来 external adapter，但这些替换点 MUST 通过明确接口接入。

#### Scenario: Source provider contributes bounded context

- **GIVEN** skill、memory、trace、tool schema 或 workspace source provider 可用
- **WHEN** context engine 组装模型请求
- **THEN** source provider 必须只通过标准 source contract 提供 bounded context candidate
- **AND** source provider 不得直接修改 LLM 请求或持久化 transcript

#### Scenario: Policy replacement controls context decisions

- **GIVEN** 系统配置了预算、裁剪、压缩或 recall policy
- **WHEN** context engine 需要决定哪些 source 进入模型请求
- **THEN** engine 必须通过 policy abstraction 做决策
- **AND** 决策必须记录到 `ContextReport`

#### Scenario: External context system remains behind adapter boundary

- **GIVEN** 用户未来接入外部上下文管理系统
- **WHEN** 外部系统返回组装后的上下文或诊断信息
- **THEN** Macaca 必须通过 adapter/anti-corruption layer 校验 schema、预算、trust boundary 和 fallback 行为
- **AND** 外部系统输出不得绕过 Macaca 的安全和预算验证

### Requirement: Macaca SHALL generate a ContextReport for model requests

Macaca SHALL 为通过 context engine 组装的每次模型请求生成 `ContextReport`，用于说明上下文来源、预算、估算 token、hash、裁剪/降级决策和 warning。

#### Scenario: Report includes source and token breakdown

- **GIVEN** 一次模型请求包含 system prompt、history、tool schema、skill、memory 或 trace source
- **WHEN** 请求通过 context engine 组装
- **THEN** `ContextReport` 必须包含 engine id、app id、session id、agent name、model、token budget 和 estimated total tokens
- **AND** `ContextReport` 必须按 source kind 记录估算 token 或大小摘要

#### Scenario: Report records decisions

- **GIVEN** context engine 做出 fallback、裁剪、跳过 source、预算超限或 warning 决策
- **WHEN** 请求组装完成
- **THEN** `ContextReport` 必须记录该决策的原因和影响
- **AND** 上层诊断 API 必须能读取这些摘要信息

#### Scenario: Report avoids full prompt leakage by default

- **GIVEN** 系统未启用显式 debug 配置
- **WHEN** `ContextReport` 被持久化或通过 API 返回
- **THEN** 报告不得默认包含完整 system prompt、完整 user prompt、完整 tool output 或完整 memory 内容
- **AND** 报告必须优先包含 source id、hash、大小、估算 token 和决策摘要

### Requirement: Prompt composition SHALL separate stable and dynamic sections

Macaca SHALL 提供 `PromptComposer` 或等价机制，将 prompt 构建为有类型 sections，并显式区分 stable 和 dynamic 内容，以支持 prompt cache 稳定性和上下文可解释性。

#### Scenario: Stable prompt hash ignores dynamic request data

- **GIVEN** 两次请求的 stable sections 相同，但 dynamic sections 中的 session metadata、time、trace 或 recall injection 不同
- **WHEN** `PromptComposer` 渲染请求并计算 hash
- **THEN** `stable_prompt_hash` 必须保持不变
- **AND** `prompt_hash` 可以反映完整请求内容变化

#### Scenario: Unknown request-specific source is dynamic

- **GIVEN** 一个 source 不能被证明是稳定、可信且跨请求不变
- **WHEN** 该 source 被加入 prompt sections
- **THEN** `PromptComposer` 必须将其归类为 dynamic section
- **AND** 该 source 不得进入 stable prefix

#### Scenario: Prompt rendering is deterministic

- **GIVEN** skill、capability、tool、workspace 或 agent-derived sections 来自无序 map/set
- **WHEN** `PromptComposer` 渲染 prompt
- **THEN** sections 必须按确定性规则排序
- **AND** 相同输入必须产生相同 stable prompt hash

### Requirement: Dynamic and untrusted context SHALL be isolated

Macaca SHALL 显式标记动态上下文和不可信上下文，防止 memory recall、workspace 文件、外部 context manager、trace event 或 tool output 被误当成系统指令。

#### Scenario: Dynamic injection is request-only

- **GIVEN** context engine 为一次请求加入 memory recall、trace snippet 或外部 context injection
- **WHEN** 请求发送给 LLM provider
- **THEN** 该 injection 必须作为 request-only dynamic context 参与本次请求
- **AND** 该 injection 不得被写回 canonical session transcript

#### Scenario: Untrusted source is fenced

- **GIVEN** source 来自 memory、workspace、trace event、tool output 或外部 context manager
- **WHEN** source 被渲染到模型上下文
- **THEN** 渲染结果必须携带 trust metadata 或明确 fence
- **AND** 系统不得把该 source 作为高优先级 system instruction 处理

### Requirement: Legacy prompt and context entry points SHALL remain searchable during migration

Macaca MUST 在迁移到 context engine facade 后保留旧 prompt/context 入口并标记 deprecated，不得立即删除，以便后续迁移和审计时查找。

#### Scenario: Deprecated legacy entry remains but is prohibited for new calls

- **GIVEN** 某个旧 prompt/context 构造入口已被 context facade 替代
- **WHEN** 迁移完成
- **THEN** 旧入口必须标记为 deprecated，并说明替代 API
- **AND** 新生产代码不得继续调用该 deprecated 入口
- **AND** 旧入口不得在本变更中删除

#### Scenario: Deprecated calls are discoverable

- **GIVEN** 开发者需要查找后续仍待迁移的 legacy context 调用
- **WHEN** 使用全文搜索查找 deprecated 注解或旧入口名称
- **THEN** 这些接口必须仍可被定位
- **AND** 迁移任务必须记录剩余调用是否为测试、兼容层或待迁移生产路径
