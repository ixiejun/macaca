# Change: 将主要消费方迁移到基于设计模式重构后的 macaca-app 抽象

## Why

`macaca-app` 已经完成第一轮基于设计模式的重构，新增了：

- `AppRuntimeBuilder`
- `ApplicationRuntimeFactory`
- `WorkflowPromptParts`
- `WorkflowPromptStrategy`
- `AppCapabilitySet`

但这些抽象目前主要还停留在 `macaca-app` crate 内部。真正的主要消费方，例如 `macaca-web`、`macaca-task`、`macaca-framework`、`macaca-cli`，仍然存在以下问题：

- 继续在调用侧重复理解 application runtime / workflow prompt / driver-tool 规则。
- 对 prompt、tool policy、capability 的解释分散，容易再次把规则写回 web/task/framework 侧。
- application-level 语义没有真正成为稳定边界，上层仍有机会绕过 `macaca-app` 新抽象。

这会削弱前一步重构的价值。必须继续把主要消费方迁移到新的 `macaca-app` 抽象上，才能让 application manifest、workflow prompt、capability、runtime startup 的解释权真正收敛回 `macaca-app`。

## What Changes

- 让 `macaca-web` 通过 `macaca-app` 的 builder/factory/prompt abstractions 消费 application runtime 和 workflow prompt，而不是继续持有平行解释逻辑。
- 让 `macaca-task` 在 planner/worker 相关 application 语义上依赖稳定的 `macaca-app` contract，而不是依赖外部字符串约定或调用侧二次推断。
- 让 `macaca-framework` 接住来自 application 层的结构化 prompt / capability / tool policy 输入，而不是只接收大段最终字符串。
- 让 `macaca-cli` 的 app startup / inspect / debug 路径复用 `macaca-app` 的 runtime 装配入口。
- 在迁移过程中保持行为 1:1 兼容，不改变现有应用的 session、trace、resume、todo、driver、MCP、skill 语义。

## Non-Goals

- 不在本 change 中重写 planner / coordinator / worker 的业务调度逻辑。
- 不在本 change 中修改 manifest schema。
- 不一次性删除所有旧 helper；旧入口允许先委托到新抽象。
- 不把 `macaca-app` 的应用语义下沉成内核级协议；本轮仍以主要消费方迁移为边界。
- 不为 `FULLSTACK-AUTODEV` 或 `NEWSROOM-AUTOWRITER` 增加任何特化分支。

## Impact

- Affected specs:
  - `macaca-app-consumer-migration`
- Affected code:
  - `macaca/crates/macaca-web`
  - `macaca/crates/macaca-task`
  - `macaca/crates/macaca-framework`
  - `macaca/crates/macaca-cli`
  - `macaca/crates/macaca-app`
- Expected risk: High
- Risk reason:
  - 这些 crate 位于 application runtime、workflow prompt、agent construction、CLI startup 的主链路。
  - 如果迁移边界处理不清晰，容易引入 trace 丢失、tool policy 偏差、prompt 行为漂移或启动语义变化。
- Compatibility requirements:
  - 新建 session 的实时 trace 推送不能退化。
  - 刷新后历史 trace 恢复不能退化。
  - planner / coordinator / worker 的 application-level tool visibility 不能退化。
  - app startup、workflow prompt、entry agent 选择、allowed_tools 解释语义必须保持兼容。

## Rollout Strategy

1. 先锁定主要消费方当前行为，并识别仍在重复解释 application 语义的位置。
2. 先迁移 `macaca-web`，因为它是最主要消费方，也是最容易把规则重新写死的地方。
3. 再迁移 `macaca-task` 的 application 语义依赖边界。
4. 再迁移 `macaca-framework` 的结构化 prompt / capability / tool-policy 接口。
5. 最后迁移 `macaca-cli`，统一 app startup / inspect / debug 路径。
6. 每一步都必须保留兼容 façade，并且以 trace / runtime / prompt 等价验证收口。
