# Change: Add standard AgentSkills runtime support

## Why

Macaca OS 的目标是支撑 7x24 小时自主运行的智能体操作系统，agent 需要可持续积累、发现、选择并执行标准化技能。当前系统已有 `SKILL.md` catalog 雏形，但没有形成完整的 skill runtime：缺少标准来源优先级、per-agent 可见性、metadata gating、session snapshot、prompt 注入、资源访问约束和 trace 可观测性，导致无法复用成熟 AgentSkills/OpenClaw 生态，也不利于用户以低干预方式扩展 agent 能力。

## What Changes

- 引入 AgentSkills-compatible `SKILL.md` 作为 Macaca OS 的标准 knowledge skill 格式。
- 建立完整 skill discovery 和 precedence：app/workspace、project `.agents`、user `.agents`、Macaca central、bundled、extra dirs。
- 建立 per-agent skill policy：agent 可配置 allowlist/denylist/disabled model invocation，默认继承 application 配置。
- 支持 `metadata.macaca`，并兼容读取 `metadata.openclaw` 的核心字段：OS、bins、env、config、primaryEnv、always、homepage、emoji、install metadata。
- 在 traced framework agent 入口统一注入 `<available_skills>`，所有 application 的所有 agent 都走同一 skill runtime。
- 在 session/run 创建时冻结 `SkillSnapshot`，刷新、resume、cron/后台运行时使用同一快照，避免运行中 skill 配置漂移。
- 提供 skill 资源读取规则：模型先看到 catalog，匹配任务后读取 `SKILL.md`，相对路径必须按 skill base dir 解析，并禁止越界读取。
- 将 skill load/filter/snapshot/invocation 纳入 trace event，保证用户可见 agent 为什么看到/使用某个 skill。
- 保留 YAML executable skills，但明确其职责与 AgentSkills knowledge skills 分离。
- 暂不实现 ClawHub/marketplace search、install、update，但预留 provider/registry 接口，后续可以接入成熟市场。

## Impact

- Affected specs: `agent-skills-runtime`
- Affected code:
  - `macaca/crates/macaca-skill/src/*`
  - `macaca/crates/macaca-app/src/model.rs`
  - `macaca/crates/macaca-sdk/src/config.rs`
  - `macaca/crates/macaca-web/src/framework_runner.rs`
  - `macaca/crates/macaca-web/src/state.rs`
  - `macaca/crates/macaca-web/src/lib.rs`
  - `macaca/crates/macaca-web/src/routes.rs`
  - `macaca/crates/macaca-framework/src/*`
  - frontend session/trace UI if exposed skill trace events require rendering
- Out of scope:
  - ClawHub 或其他 marketplace 的搜索、安装、更新 CLI/API
  - 自动生成 skill 的 Skill Workshop 类能力
  - 一次性迁移全部现有 persona/tool 文档为 skills
