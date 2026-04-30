## 1. Preparation

- [x] 1.1 阅读 `refactor-macaca-app-patterns` 的 proposal/design/tasks，确认已经落地的新抽象与未完成边界
- [x] 1.2 运行 GitNexus impact：`FrameworkRunner` upstream，记录风险和直接调用者
- [x] 1.3 运行 GitNexus impact：`WorkflowEngine` upstream，记录风险和直接调用者
- [x] 1.4 运行 GitNexus impact：`AppRuntime` upstream，记录风险和直接调用者
- [x] 1.5 识别 `macaca-web`、`macaca-task`、`macaca-framework`、`macaca-cli` 中仍在重复解释 application 语义的代码点

## 2. macaca-web Migration

- [x] 2.1 将 `macaca-web` 的 application runtime 消费入口迁移到 `AppRuntimeBuilder` / `ApplicationRuntimeFactory` 或兼容 façade
- [x] 2.2 将 `macaca-web` 的 workflow prompt 消费入口迁移到 `WorkflowPromptStrategy` / `WorkflowPromptParts` 或兼容 façade
- [x] 2.3 清理 `macaca-web` 中与 application-level tool/driver policy 重复的解释逻辑，改为消费 `macaca-app` 结构化输入
- [x] 2.4 保留现有 web façade 名称，对外行为保持兼容
- [ ] 2.5 验证新建 session、刷新恢复、实时 trace、历史 trace 行为不退化

## 3. macaca-task Migration

- [x] 3.1 梳理 planner/worker 侧依赖的 application-level 语义
- [x] 3.2 将相关 application 解释入口迁移到 `macaca-app` contract 或兼容 façade
- [ ] 3.3 保持 task board、dependency、review、resume 行为兼容

## 4. macaca-framework Migration

- [x] 4.1 让 framework-level primitive 接收结构化 application prompt / capability / tool policy 输入
- [x] 4.2 避免 framework 继续只依赖最终拼好的 prompt 字符串
- [x] 4.3 保持 traced construction、tool visibility、trace event 行为兼容

## 5. macaca-cli Migration

- [x] 5.1 梳理 CLI 中 app startup / inspect / debug 的 application runtime 使用点
- [x] 5.2 迁移到 `AppRuntimeBuilder` / `ApplicationRuntimeFactory` 或兼容 façade
- [x] 5.3 保持现有命令行为和输出兼容

## 6. Verification

- [x] 6.1 运行 `cargo fmt`
- [x] 6.2 运行 `cargo test -p macaca-app`
- [x] 6.3 运行 `cargo test -p macaca-web`
- [x] 6.4 运行 `cargo test -p macaca-task`
- [x] 6.5 运行 `cargo check -p macaca-framework -p macaca-web -p macaca-task -p macaca-cli`
- [ ] 6.6 运行至少一轮真实 session 联调，验证实时 trace、刷新恢复、历史事件恢复与增量推送
- [ ] 6.7 运行 `gitnexus_detect_changes(scope: "all")`，确认影响范围符合预期
- [x] 6.8 更新 checklist，只在真实完成后勾选
