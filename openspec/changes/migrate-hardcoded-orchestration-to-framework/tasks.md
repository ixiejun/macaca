## 1. Exploration and Boundaries

- [x] 1.1 盘点所有 legacy `AgenticLoop` 与 direct `llm.chat` 入口
- [x] 1.2 标记所有基于 agent 名的 tool/orchestration 分支
- [x] 1.3 标记所有 application-specific 纪律下沉到 OS substrate 的位置

## 2. Execution Substrate Migration

- [x] 2.1 将 legacy agent execution 统一迁到 framework wrapper
- [x] 2.2 禁止新增非 framework 的业务执行入口
- [x] 2.3 清理重复的 execution/event glue

## 3. Tool Policy Migration

- [x] 3.1 引入 capability/config-driven tool policy
- [x] 3.2 替换 `coordinator/planner/worker` 名称分支
- [x] 3.3 兼容现有 fullstack-autodev app 配置

## 4. Orchestration and Session Migration

- [x] 4.1 将 app-specific gating 从 OS substrate 迁到 app workflow/policy
- [x] 4.2 将 planner/review 状态更多下沉到 framework planning primitive
- [x] 4.3 演进 session/trace/resume 为 framework-level primitive

## 5. Verification

- [x] 5.1 端到端验证 fullstack-autodev 主链路
- [x] 5.2 验证新增 application 不需要 OS 源码里加角色名 if/else
- [x] 5.3 cargo check / tests / migration audit 文档同步
