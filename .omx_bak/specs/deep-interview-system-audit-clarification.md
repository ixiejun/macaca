# Deep Interview Spec — system-audit-clarification

## Metadata
- Profile: standard
- Rounds: 5
- Final ambiguity: 0.098
- Threshold: 0.20
- Context type: brownfield
- Context snapshot: `.omx/context/system-audit-clarification-20260407T064738Z.md`
- Transcript: generated under `.omx/interviews/`

## Clarity Breakdown
| Dimension | Score |
|---|---:|
| Intent | 0.97 |
| Outcome | 0.88 |
| Scope | 0.92 |
| Constraints | 0.85 |
| Success | 0.82 |
| Context | 0.90 |

## Intent
将 `macaca/docs/SYSTEM_AUDIT.md` 从单纯的审计报告，澄清为**后续重构的执行依据**的一部分；同时补齐当前文档缺失的“系统是什么、为什么存在、有哪些核心功能与设计要点”的上层说明，使后续重构不再在目标定义不清的前提下推进。

## Desired Outcome
形成两层文档结构：
1. **系统定义 / 目标系统说明文档**：说明这个系统是什么、解决什么问题、想成为怎样的系统、核心功能、关键模块、整体架构、任务执行完整链路。
2. **执行审计 / 重构行动文档**（保留 `SYSTEM_AUDIT.md` 的角色，但收窄职责）：基于目标系统定义，对当前实现进行审计、优先级排序，并给出服务于重构的执行依据。

## In Scope
- 澄清系统目标与产品/平台定位
- 澄清系统解决的问题与目标用户体验方向
- 澄清系统详细结构、关键模块及其职责
- 澄清任务执行的完整链路
- 重排当前审计问题优先级
- 改写或删除现有 `SYSTEM_AUDIT.md` 中不适合作为执行依据的表述
- 将现有混合文档拆成“系统定义”与“执行审计”两层

## Out of Scope / Non-goals
- 不只是润色现有文档措辞
- 不维持“系统定义”和“审计行动”混在同一文档里的现状
- 不让系统继续滑向不可维护的“尸山代码”状态
- 不把系统定位为玩具项目或仅用于演示的样例系统

## Decision Boundaries
OMX 可直接决定：
1. 当前审计问题的优先级重排
2. 现有文档结论表述的改写/删除
3. 新增“系统定义/目标系统说明”文档，并将 `SYSTEM_AUDIT.md` 收窄为执行审计文档

## Constraints
- 当前需求处于 deep-interview 产物阶段，不直接进入实现
- 后续文档必须服务于“可执行的重构依据”，而非只做静态分析汇总
- 质量约束必须围绕可维护性：控制文件膨胀、降低逻辑复杂度、减少硬编码、恢复清晰设计边界、提升实现优雅性
- 系统目标应对齐“面向大规模真实使用、体验良好的 Agent OS”，而非玩具工程

## Testable Acceptance Criteria
读者第一次接触该项目，读完两份文档后，必须能明确回答：
1. 这是一个什么项目
2. 它解决什么问题
3. 它的系统详细结构是什么
4. 每个核心模块在架构中的功能是什么
5. 任务执行的完整链路是什么

此外，执行审计文档应满足：
6. 明确区分“系统目标/定义”与“当前实现审计/重构建议”
7. 当前审计项按重构价值与风险重新排序，而不是沿用原顺序
8. 每条关键重构建议都能回连到目标系统定义中的某个设计或质量目标

## Assumptions Exposed + Resolutions
- Assumption: 当前 `SYSTEM_AUDIT.md` 可以单独承担全部职责
  - Resolution: 不可以，应拆为两层文档
- Assumption: 只要补充一些表述就足够
  - Resolution: 不够，必须补充系统定义层，才能让审计层成为执行依据
- Assumption: 现有问题只是文档组织问题
  - Resolution: 不只是组织问题，背后反映出代码结构已出现不可维护倾向

## Pressure-pass Findings
- Revisited answer: “不希望它变成什么”
- Finding: 用户的真实硬性非目标是防止系统继续演化为不可维护的尸山代码
- Practical implication: 后续 planning / 重构必须把可维护性、复杂度控制、去硬编码、架构边界恢复当成一等目标

## Brownfield Evidence vs Inference
### Evidence
- `macaca/docs/SYSTEM_AUDIT.md` 已存在，内容混合了架构审计、前端审计、技术债总结与重构建议
- 仓库内未发现其他文件直接引用 `SYSTEM_AUDIT.md`
- 文档内明确点名了若干当前问题，如 `macaca-web/src/routes.rs` 巨型文件、AppState God Object、多个 "coordinator" 硬编码等

### Inference
- 将该文档变为“重构执行依据”需要先补齐系统定义层，否则执行依据会缺少上位目标
- 两层拆分会比继续维护单份混合文档更利于后续 planning 与执行对齐

## Technical Context Findings
- 目标文档位于 `macaca/docs/SYSTEM_AUDIT.md`
- 同目录存在相关审计/设计资料，例如 `system-integration-audit.md`, `task-session-isolation-plan.md`
- 当前审计文档已覆盖一部分底层事实，但未充分表达系统总目标、模块设计核心、端到端任务链路

## Recommended Handoff
### Recommended: `$ralplan`
- Invocation: `$plan --consensus --direct .omx/specs/deep-interview-system-audit-clarification.md`
- Why: 当前需求已经过澄清，下一步最适合进入结构化规划，把“两层文档”的最终结构、章节、与验证标准固化成 planning artifacts

## Residual Risk
- 低风险：系统的理想愿景已经明确，但“目标系统说明文档”的最终写法仍存在表述与组织层面的设计空间；这更适合在 ralplan 阶段收敛，而不是直接进入实现
