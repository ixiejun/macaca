# Macaca Agent OS 设计模式渐进式重构计划

本文档集面向 Macaca Agent OS 后端所有 crate，目标是在不破坏现有功能的前提下，用 `docs/design_patterns.md` 中列出的设计模式逐步替换重复、硬编码、职责混杂和扩展困难的实现。

## 范围

本轮只做方案落盘，不改运行代码。覆盖以下后端 crate：

- `macaca-agent`
- `macaca-app`
- `macaca-cli`
- `macaca-driver`
- `macaca-framework`
- `macaca-gateway`
- `macaca-integration-tests`
- `macaca-ipc`
- `macaca-kernel`
- `macaca-llm`
- `macaca-memory`
- `macaca-persist`
- `macaca-proto`
- `macaca-runtime`
- `macaca-runtime-host`
- `macaca-sdk`
- `macaca-skill`
- `macaca-task`
- `macaca-tools`
- `macaca-web`

## 设计约束

- 每次重构只做一个很小的、可回滚的切片。
- 行为必须 1:1 还原，先抽象再替换，不在同一轮里改变业务语义。
- 任何跨 crate 的抽象必须先在调用侧做兼容适配，再逐步迁移实现。
- Trace、session、resume、task todo、driver、MCP、skill 是 Agent OS 的基础能力，不能因为重构降低可观测性。
- 不以 `FULLSTACK-AUTODEV` 或 `NEWSROOM-AUTOWRITER` 写专门逻辑，应用差异必须通过 manifest、capability、tool policy、planner prompt 或 framework primitive 表达。

## 模式使用原则

- Factory Method / Abstract Factory：用于 provider、driver、tool、runtime、application bootstrap 的创建扩展点。
- Builder：用于配置对象、runtime context、server bootstrap、agent 构建参数，替代长参数列表。
- Adapter：用于外部协议、旧接口与 `macaca-framework` 原语之间的转换。
- Bridge：用于运行时通道、driver 协议、MCP transport、IPC transport 的实现解耦。
- Composite：用于消息、tool group、pipeline、workflow、agent tree。
- Decorator：用于 trace、retry、rate-limit、permission、middleware 等横切能力。
- Facade：用于隐藏复杂子系统启动、注册、持久化、调度细节。
- Proxy：用于远程 LLM、driver process、MCP server、vector store、动态库边界。
- Chain of Responsibility：用于 tool middleware、permission approval、provider fallback、event enrichment。
- Command：用于 tool call、task action、driver action、IPC message、CLI command。
- Mediator：用于 task board、loop manager、gateway、kernel orchestrator 的参与者协调。
- Memento：用于 session、event log、checkpoint、plan notebook、resume state。
- Observer：用于 SSE、event bus、trace sink、status watcher。
- State：用于 agent lifecycle、todo lifecycle、goal lifecycle、driver session lifecycle。
- Strategy：用于 planner/decomposer/reviewer/provider/router/scheduler/permission/formatter。
- Template Method：用于 agent loop、plan loop、worker loop、goal evaluation 等固定流程加可替换步骤。
- Visitor：用于 `ContentBlock`、proto event、trace event、message payload 的展示和转换。

## 文档索引

- [macaca-agent](macaca-agent.md)
- [macaca-app](macaca-app.md)
- [macaca-cli](macaca-cli.md)
- [macaca-driver](macaca-driver.md)
- [macaca-framework](macaca-framework.md)
- [macaca-gateway](macaca-gateway.md)
- [macaca-integration-tests](macaca-integration-tests.md)
- [macaca-ipc](macaca-ipc.md)
- [macaca-kernel](macaca-kernel.md)
- [macaca-llm](macaca-llm.md)
- [macaca-memory](macaca-memory.md)
- [macaca-persist](macaca-persist.md)
- [macaca-proto](macaca-proto.md)
- [macaca-runtime](macaca-runtime.md)
- [macaca-runtime-host](macaca-runtime-host.md)
- [macaca-sdk](macaca-sdk.md)
- [macaca-skill](macaca-skill.md)
- [macaca-task](macaca-task.md)
- [macaca-tools](macaca-tools.md)
- [macaca-web](macaca-web.md)

## GitNexus 状态说明

本轮曾按项目要求尝试运行 `npx gitnexus analyze` 刷新索引，但命令长时间停留在 npm 安装阶段，超过数分钟无有效分析进度后已中止。以下文档基于源码结构扫描、现有图谱概览和重点模块阅读形成。后续进入代码实现阶段前，仍需要按 `AGENTS.md` 要求重新刷新 GitNexus，并对将要修改的具体 symbol 逐一运行 impact analysis。

