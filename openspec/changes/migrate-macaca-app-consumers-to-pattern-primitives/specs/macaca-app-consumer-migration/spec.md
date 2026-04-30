## ADDED Requirements

### Requirement: 主要消费方 SHALL 通过 macaca-app 抽象消费 application runtime

系统 SHALL 让 `macaca-web`、`macaca-task`、`macaca-framework`、`macaca-cli` 通过 `macaca-app` 暴露的 builder / factory / compatibility façade 消费 application runtime，而不是继续保留平行的 application runtime 解释逻辑。

#### Scenario: Web 通过 app runtime 抽象启动和读取应用

- **GIVEN** `macaca-web` 需要加载应用、读取 workflow 或准备 runtime 上下文
- **WHEN** 消费方迁移完成
- **THEN** `macaca-web` SHALL 通过 `macaca-app` 的 builder / factory / façade 获取 application runtime 结果
- **AND** 不再在 web 侧保留平行的 application startup 解释逻辑

#### Scenario: CLI 通过统一 runtime abstraction 访问应用

- **GIVEN** `macaca-cli` 需要执行 app startup、inspect 或 debug 路径
- **WHEN** 消费方迁移完成
- **THEN** CLI SHALL 通过 `macaca-app` 的 runtime abstraction 或兼容 façade 访问应用
- **AND** CLI 与 web/runtime 的 application 解释语义 SHALL 保持一致

### Requirement: 主要消费方 SHALL 通过 macaca-app 抽象消费 workflow prompt 与 tool policy

系统 SHALL 将 workflow prompt、driver/tool 规则、application-level tool visibility 的解释权收敛到 `macaca-app`，主要消费方只能消费结构化结果或兼容 façade。

#### Scenario: Web 不再重写 workflow prompt 规则

- **GIVEN** `macaca-web` 需要构造 coordinator、planner 或 worker 的 application-level prompt 上下文
- **WHEN** 消费方迁移完成
- **THEN** web SHALL 通过 `WorkflowPromptStrategy`、`WorkflowPromptParts` 或兼容 façade 获取 prompt 结果
- **AND** web SHALL NOT 再自行写死 application-level driver/tool prompt 规则

#### Scenario: Framework 接收结构化 prompt 和 tool policy 输入

- **GIVEN** `macaca-framework` 需要执行 framework-level agent construction 或 execution primitive
- **WHEN** 消费方迁移完成
- **THEN** framework SHALL 接收来自 `macaca-app` 的结构化 prompt / tool policy / capability 输入，或其兼容 façade
- **AND** framework SHALL NOT 仅依赖最终拼好的单一 prompt 字符串作为唯一 application 语义来源

### Requirement: Task-side application 语义 SHALL 依赖稳定 contract

系统 SHALL 让 `macaca-task` 通过稳定的 application contract 消费 planner/worker 所需的 application-level 语义，而不是依赖调用侧二次拼装或字符串约定。

#### Scenario: Planner side 通过 application contract 获取语义

- **GIVEN** planner decomposition 或 review 需要读取 workflow、tool visibility 或 application-level 约束
- **WHEN** 消费方迁移完成
- **THEN** `macaca-task` SHALL 通过稳定的 `macaca-app` contract 或兼容 façade 获取这些语义
- **AND** 不再依赖调用侧再把 application 规则翻译成临时字符串

#### Scenario: Worker side 通过 application contract 获取语义

- **GIVEN** worker execution 需要理解 application-level capability、workflow context 或 tool policy
- **WHEN** 消费方迁移完成
- **THEN** `macaca-task` SHALL 通过稳定的 `macaca-app` contract 或兼容 façade 获取这些语义
- **AND** task board、dependency、review、resume 行为 SHALL 保持兼容

### Requirement: 迁移 SHALL 保持 trace 与 session 行为兼容

系统 SHALL 在主要消费方迁移期间保持实时 trace、历史 trace 恢复、session 刷新恢复和事件增量推送行为兼容。

#### Scenario: 新建 session 实时 trace 不退化

- **GIVEN** 用户新建一个 session 并发送新的 application 请求
- **WHEN** 主要消费方迁移完成
- **THEN** 浏览器 SHALL 无需刷新即可收到 coordinator、planner、worker 等实时 trace 事件
- **AND** trace event 类别和基本路由语义 SHALL 与当前行为兼容

#### Scenario: 刷新后历史 trace 恢复不退化

- **GIVEN** 一个 session 已经产生并持久化了一批 trace 事件
- **WHEN** 用户刷新浏览器并重新进入该 session
- **THEN** 历史 trace SHALL 能从持久化事件中恢复
- **AND** 后续增量 trace SHALL 继续实时推送
