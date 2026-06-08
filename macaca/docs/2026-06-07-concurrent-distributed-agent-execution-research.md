# 并发与分布式 Agent 执行调研

> 日期：2026-06-07
> 状态：调研与实施方案，尚不是已批准的 OpenSpec 变更
> 范围：支持单个 application 内同时发起多个 session 级任务，并支持在集群中调度互相隔离的 agent 执行通道。

## 执行摘要

Macaca 已经具备一部分隔离基础：`TodoItem`、`TodoGoal`、`TaskBoard`、`TaskSpace`、`TodoStore` 已经携带 `session_id`，并能按单 session 或跨 session 查询。因此当前问题不只是“补一个 session_id”。真正缺失的是一个服务化的执行控制面：它需要允许同一个 application 下存在多个独立的 `ExecutionRun`，为每个 run 分配本机或远程 worker lease，执行资源/策略/并发限制，持续 checkpoint，并暴露可追踪状态，同时保证会话、prompt、任务面板、agent workspace 不互相污染。

推荐方向：

- 将用户发起的每一次任务建模为持久化的 `ExecutionRun`，作用域为 `(application_id, session_id, task_id, run_id)`。
- 将“同一 application/session UI 只能有一个活跃任务”的语义从 shell 状态中移出，放到 `Execution Control Service` 和 `Agent Execution Service` 的通用控制面里。
- microkernel 只保留 identity、service registry、trace/audit、policy、scheduler/resource primitives、session/task 状态契约。
- 在 system service 层提供 cluster-aware 的 `Placement Service` 策略，根据声明资源、实时负载、亲和性、数据局部性、健康状态、tenant/app policy 选择 worker node；不要把调度语义放进 Web/CLI/frontend。
- 每个 agent lane 都在隔离 worker sandbox 中运行，拥有独立 conversation state、workspace/session mount、tool lease、cancellation token、checkpoint stream、trace context。
- 先完成本机多 run 并发，再加入远程 worker lease，最后加入 bin-packing、autoscaling、rebalance。不要一开始就在 Macaca 内部实现一个完整 Kubernetes 级 scheduler。

## 宪法约束

本设计必须遵循：

- `docs/macaca-os-architecture-governance.md`
- `docs/macaca-os-microkernel-boundaries.md`
- `docs/macaca-os-serviceization-allowlist.md`

由此得到的约束：

- kernel 可以知道 `application_id`、`session_id`、`task_id`、`run_id`、`agent_id`、`service_id`、trace id、policy decision、resource reservation、lease state，但不能执行 agent loop，也不能选择具体 provider。
- planning、task execution、review、recovery、retry、scheduler、driver、skill、MCP、LLM、memory、context、application lifecycle 都属于 system service。
- Web/CLI/frontend 只能创建 session/run、渲染进度、请求取消、订阅事件，不能拥有队列、并发、placement、worker lease、recovery 语义。
- 分布式执行必须是可选模块：单机 Macaca host 仍然能启动；未启用远程 provider 时，应返回结构化 `unavailable` 或 `unsupported`，而不是崩溃、卡死或假成功。
- OS 层代码不能硬编码 app name、workflow name、agent name、driver name、provider name 或业务领域。

## 当前 Macaca 基线

当前已有证据：

- `crates/services/macaca-task/src/todo_store.rs` 使用 `todo/{app_id}/{session_id}/{agent}/{task_id}` 存储任务，使用 `goal/{app_id}/{session_id}/{goal_id}` 存储 goal。
- `TaskBoard` 和 `TaskSpace` 已经携带 `session_id: Option<String>`。
- `TaskBoard::claim_next_task` 已按 `session_id` 分组，并在每个 session 内应用顺序约束，这意味着不同 session 天然可以独立推进。
- `SchedulerTargetCommand::AgentExecution` 已经包含 `application_id`、`session_id`、可选 `task_id`、`target_agent`、execution intent 和安全 payload ref。
- `AgentExecutionPort` 在 kernel 中是 provider-neutral 且可热替换，这与后续 service-client backed execution adapter 兼容。

当前缺口：

- 数据隔离已经存在，但还没有明确的集群级 `ExecutionRun` lease 模型，无法让同一个 application 下的多个 run 在不同 worker 上独立推进。
- 跨 session 的 `claim_next_task` 目前偏向最新 session，并且按 agent/session sequence 一次 claim 一个任务。这对公平性有价值，但还不是完整的 admission、placement、concurrency、resource-control 契约。
- 当前 autonomy/scheduler 文档主要覆盖 recurring job 和 heartbeat lane，还没有为用户即时发起的 ad hoc agent execution 定义通用远程 worker placement 层。

## 外部调研

### Durable Execution 与 Worker Queue

Temporal 将 durable workflow state 与 worker execution 分离。worker 轮询 task queue，多个 worker process 可以消费同一个 task queue，以获得可用性和水平扩展能力。Temporal 官方文档强调 crash-proof execution、故障后恢复、worker 轮询 task queue：

- <https://docs.temporal.io/>
- <https://api-docs.temporal.io/>
- <https://github.com/temporalio/temporal/blob/main/docs/architecture/README.md>

对 Macaca 的启发：

- 以 durable event/checkpoint log 作为事实来源，而不是依赖内存 session state。
- 将 work 表达为可排队的 typed command/result。
- 允许多个 worker 轮询兼容 queue，不要通过单个 UI session lock 分配任务。
- tool call 需要 idempotency key 和 side-effect receipt，否则 retry 会重复外部副作用。

### 资源感知的集群调度

Kubernetes scheduler 会筛选可行节点、排序节点并绑定 Pod。它将 resource requests 作为调度输入，支持 node affinity、taints/tolerations、topology spread，以及可插拔 scheduling framework：

- <https://kubernetes.io/docs/concepts/scheduling-eviction/kube-scheduler/>
- <https://kubernetes.io/docs/reference/command-line-tools-reference/kube-scheduler>
- <https://kubernetes.io/docs/concepts/scheduling-eviction/assign-pod-node/>
- <https://v1-35.docs.kubernetes.io/docs/concepts/scheduling-eviction/scheduling-framework>
- <https://kubernetes.io/docs/concepts/scheduling-eviction/node-pressure-eviction>

Nomad 强调 feasible node filtering、bin packing、constraint、affinity，以及显式 job resource block：

- <https://developer.hashicorp.com/nomad/docs/concepts/architecture>
- <https://developer.hashicorp.com/nomad/docs/job-specification/resources>
- <https://developer.hashicorp.com/nomad/docs/job-specification/affinity>

对 Macaca 的启发：

- placement 应建模为 filter -> score -> reserve -> bind。
- 同时使用声明资源和实时观测负载；不要只根据 CPU 使用率做调度。
- 硬约束和软偏好必须分离。
- 需要 node-pressure threshold 和 graceful drain/cancel 语义。
- placement strategy 应可插拔，不能写死调度规则。

### 分布式 Task 与 Actor Runtime

Ray 提供 logical resource、custom resource、actor/task scheduling、placement group，以及基于 resource demand 的 autoscaling：

- <https://docs.ray.io/en/latest/ray-core/scheduling/resources.html>
- <https://docs.ray.io/en/latest/ray-core/scheduling/placement-group.html>
- <https://docs.ray.io/en/latest/cluster/vms/user-guides/configuring-autoscaling.html>
- <https://docs.ray.io/en/latest/ray-core/fault_tolerance/actors.html>

Dask 将计算建模为 task graph，并能在 worker 断连后重路由 pending work；但其文档也说明，若 scheduler 本身失败，未持久化的 ongoing computation state 会丢失：

- <https://docs.dask.org/en/stable/scheduling.html>
- <https://distributed.dask.org/en/latest/resilience.html>
- <https://docs.dask.org/en/stable/how-to/debug.html>

Celery 使用 routing key/queue，worker 可以只消费特定队列，这对区分慢任务/快任务、CPU/IO 类任务有价值：

- <https://docs.celeryq.dev/en/latest/userguide/routing.html>

Akka Cluster Sharding 与 Orleans virtual actor 都体现了一个重要原则：用逻辑 identity 寻址 stateful execution，同时隐藏物理位置：

- <https://doc.akka.io/libraries/akka-core/current/typed/cluster-sharding.html>
- <https://doc.akka.io/guide/concepts/akka-cluster.html>
- <https://doc.akka.io/libraries/akka-core/current/typed/fault-tolerance.html>
- <https://dotnet.github.io/orleans/docs/grains/reentrancy.html>

对 Macaca 的启发：

- 每个并发 agent lane 必须有逻辑 identity，并按 identity 路由，而不是按当前机器路由。
- 对 stateful agent session，健康时优先 sticky ownership；故障时从 checkpoint 在其他节点恢复。
- queue routing 应按 capability/resource profile 区分，不能按 application name 区分。
- actor supervision 很适合映射到 per-run cancellation/retry/recovery。

### Agent Runtime Persistence 与 Topic 隔离

AutoGen Core 使用 agent id、topic id、subscription 隔离消息投递范围：

- <https://microsoft.github.io/autogen/stable/user-guide/core-user-guide/core-concepts/topic-and-subscription.html>
- <https://microsoft.github.io/autogen/stable/reference/python/autogen_core.html>

LangGraph 将 assistant、thread、run 分离；thread 是持久化 conversation 容器，checkpoint 支持恢复：

- <https://docs.langchain.com/langgraph-platform/use-threads>
- <https://langchain-ai.github.io/langgraph/cloud/concepts/threads/>
- <https://docs.langchain.com/oss/python/langgraph/use-subgraphs>

对 Macaca 的启发：

- 保留 `session_id` 作为持久 conversation/task scope，并新增 `run_id` 表示每次 execution attempt。
- 事件路由可以采用 topic/source 风格：
  `execution.run/{application_id}/{session_id}/{run_id}`。
- mutable prompt/context/tool state 不能跨 run 共享；如需共享，必须通过 Memory/Context service，并带有明确 scope 和 policy。

## 设计模式适配

- Facade：暴露 `SystemExecutionClient`、`SystemPlacementClient` 和 focused SDK clients 给 shell/application。
- Command：enqueue、lease、heartbeat、checkpoint、cancel、resume、result 都是 typed command/result。
- Strategy：placement policy、queue routing、retry、recovery、load scoring 都是可替换策略。
- Adapter/Bridge：local worker、remote worker、Kubernetes-backed worker、plugin-backed worker、unavailable worker provider 实现同一 service contract。
- Decorator：trace、policy、resource reservation、budget、entitlement、metering、audit 包裹每次 execution call。
- State：`ExecutionRun`、`WorkerLease`、`PlacementDecision`、`RunAttempt` 都是显式状态机。
- Observer：session/run events 可订阅，供 shell、audit、replay、diagnostics 使用。
- Memento：checkpoint、side-effect receipt、run snapshot 支持恢复。
- Specification：placement constraints、app capability declaration、node eligibility、resource admission 应是可执行规则。
- Abstract Factory：runtime-host composition root 负责实例化 provider adapter；低层不得构造 provider。

## 备选方案

### 方案 A：仅支持本机 Session 级并发

只支持单机。移除 per-application active-run lock，创建多个 session-scoped task space，并用本机 worker lane 与 resource semaphore 执行。

优点：

- 实现风险最低。
- 可以复用现有 `session_id` 数据模型。
- 能快速验证用户可见的并发体验。

缺点：

- 不解决集群 placement。
- 如果 resource gate 不完善，容易压垮单机。

结论：作为 Phase 1 使用，但不是最终架构。

### 方案 B：Durable Queue + 本机/远程 Worker Poller

引入 `ExecutionRun` 记录和 typed work queue。worker 注册 capability/resource profile，轮询兼容任务，获取 lease，执行并 checkpoint。本机 worker 和远程 worker 使用同一个 service contract。

优点：

- 与 Temporal/Celery/Dask 的成熟模式一致。
- 符合 Macaca serviceization。
- 能在不改变 application/shell contract 的情况下加入远程执行。

缺点：

- 需要认真设计 idempotency、lease expiry、replay、side-effect receipt。
- 状态机复杂度高于纯本机方案。

结论：推荐作为核心方案。

### 方案 C：一开始就构建完整 Cluster Scheduler

直接实现 Kubernetes/Nomad 式 scheduler，包括 node inventory、scoring、gang scheduling、autoscaling、preemption、live migration。

优点：

- 长期控制力最强。

缺点：

- 复杂度和风险过高。
- 在本机多 run 语义尚未稳定前容易过度设计。
- agent process 的 live migration 并不现实，除非所有状态已经可 checkpoint。

结论：不建议作为首个实现。应增量引入其中的成熟机制。

### 方案 D：将 Placement 外包给 Kubernetes/Nomad/Ray

Macaca 创建 worker pod/job/actor，把资源调度交给外部 orchestrator。

优点：

- 复用成熟集群调度能力。
- 适合大型部署。

缺点：

- 仍然必须满足 optional module absent-safe。
- Macaca 仍然需要 execution lease、state、trace、policy、checkpoint 语义。
- 如果 base OS 语义绑定到某个 orchestrator，会违反宪法。

结论：可以作为 `PlacementService`/`WorkerProvider` 后面的可选 provider，而不能成为 base behavior。

## 推荐目标架构

```text
Web / CLI / Frontend shells
  -> SystemFacade / focused execution clients
  -> ServiceRuntime decorators
       trace + policy + resource + budget + entitlement + audit
  -> Execution Control Service
       run admission, state machine, cancellation, checkpoint refs
  -> Placement Service
       filter, score, reserve, bind worker lease
  -> Agent Execution Service
       local/remote/plugin/unavailable worker providers
  -> Worker Runtime
       isolated agent lane, tools, driver, LLM, memory/context calls
  -> Checkpoint/Event/Audit Stores
```

### 核心实体

- `ExecutionRunId`：一次用户可见的执行 run。
- `RunAttemptId`：retry/recovery 中的一次具体 attempt。
- `WorkerNodeId`：本机或远程 worker host 的稳定身份。
- `WorkerLeaseId`：将某个 run attempt 绑定到 worker node/provider 的租约。
- `ExecutionScope`：`(tenant_id, application_id, session_id, task_id?, run_id)`。
- `ExecutionProfile`：声明 capability、agent profile、resource hint、isolation class、timeout、max retries、priority、locality preference。
- `PlacementDecision`：accepted/rejected decision、feasible candidates、selected node、strategy name、score summary、trace id、安全 reason code。
- `ExecutionCheckpointRef`：指向 bounded state 的持久引用，不能包含 raw prompt 或 provider payload。
- `SideEffectReceipt`：idempotency key、tool/capability id、status、安全 result ref、retry safety classification。

### Run 状态机

```text
Requested
  -> Admitted
  -> Queued
  -> Placing
  -> Leased
  -> Starting
  -> Running
  -> Checkpointing
  -> Completing
  -> Succeeded

Failure/cancel branches:
  Requested|Admitted|Queued|Placing -> Rejected
  Leased|Starting|Running -> Cancelling -> Cancelled
  Running|Checkpointing -> Recovering -> Queued|Failed
  Any non-terminal -> Expired
```

规则：

- 每次状态迁移都必须发出 sanitized trace/audit event。
- 每个外部副作用都必须有 receipt，或显式标记为 non-idempotent。
- lease expiry 不等于可以安全 retry；retry policy 必须检查 receipt 和 checkpoint。
- cancellation 作用域必须是 `run_id`。停止一个 run 不能取消其他 session 或共享 application-level scheduler lane。

### Placement 模型

placement 应该是 strategy pipeline：

1. `Filter`：节点健康、provider 支持所需 capability、tenant/app policy 允许、资源可行、isolation class 支持、secret/capability 可安全下发。
2. `Score`：可用 CPU/memory/GPU、queue backlog、active run count、近期错误率、数据/workspace 局部性、session stickiness、网络延迟、能耗/成本策略、公平性。
3. `Reserve`：创建带 TTL 的资源 reservation。
4. `Bind`：创建 worker lease 并分发 typed `AgentExecutionCommand`。
5. `Observe`：heartbeat lease，流式 checkpoint/event，计量用量。
6. `Recover`：当 heartbeat 丢失或 node pressure 出现时，根据 run policy cancel、retry 或 requeue。

Macaca 至少应支持这些 placement strategy：

- `LocalOnly`：开发和默认安装使用的单机 provider。
- `LeastLoaded`：基于 resource snapshot 的基础集群 provider。
- `BinPack`：聚合低优先级任务，为高优先级任务保留 headroom。
- `Spread`：分散相关的高风险或高资源 run。
- `StickySession`：健康时优先使用该 session 上一次所在节点。
- `CapabilityAware`：按声明的 tool/driver/LLM/context capability 路由。

任何 strategy 都不能按具体 application name 或业务 workflow 分支。

### 隔离要求

每个并发 lane 必须拥有：

- 独立 `run_id`、`trace_id`、cancellation token、event stream、checkpoint namespace。
- session-scoped task board/tool context。
- 独立 conversation/context window；共享 memory 必须通过 Memory/Context service，并带有 scope 与 policy。
- workspace/resource mount policy。共享 filesystem 写入必须使用 lease、transactional patch 或显式冲突检测。
- tool execution idempotency 和 side-effect receipt。
- per-run budget 与 rate limit。
- per-run log/snapshot，且必须经过相同 audit policy 清洗。

这能避免“两个任务都是 frontend agent”导致互相污染：agent type 只是 capability/profile，不是隔离边界。真正隔离边界是 run scope。

## 实施计划

### Phase 0：Spec 与 Inventory

- 创建 OpenSpec change，建议 `add-concurrent-distributed-agent-execution`。
- 检查 `execution-control-service`、`service-runtime`、`sdk-system-facade` 相关既有 specs。
- 盘点 shell 中的 active-run/session lock 和 execution queue 假设。
- 编辑符号前必须运行 GitNexus impact analysis。

### Phase 1：本机多 Run Session 并发

- 在 `macaca-proto` 定义 `ExecutionRun`、`RunAttempt`、`WorkerLease`、run-state DTO。
- 增加 Execution Control service commands：
  `run.start`、`run.cancel`、`run.get`、`run.list`、`run.checkpoint.append`、`run.event.subscribe`、`run.resume`。
- Web/CLI 每次用户发起任务都创建新 run，而不是被一个 active application task 阻塞。
- 在所有执行路径传递 `session_id` 与 `run_id`，保留 session/task board 隔离。
- 增加本机 resource semaphore，覆盖 CPU-ish、memory-ish、tool、driver、LLM concurrency budget。

验证：

- 同一 application 下两个不同 session 的任务可并发执行。
- 取消一个 run 不影响另一个 run。
- task board、event stream、checkpoint、trace id 互相独立。

### Phase 2：Durable Lease 与 Recovery

- 增加 lease TTL、heartbeat、attempt retry、checkpoint ref、side-effect receipt。
- 将 worker execution 改为 lease-owned state transition。
- 增加崩溃恢复：expired running lease 进入 `Recovering`，再根据 retry policy 和 receipt 进入 `Queued`、`Failed` 或 `NeedsHuman`。
- 增加 replay diagnostics，只展示安全 event/checkpoint ref，不展示 raw payload。

验证：

- 杀掉本机 worker 后，run 能恢复或结构化失败。
- replay audit 能展示 admission、lease、start、checkpoint、failure、retry。
- non-idempotent side effect 不会被静默重复执行。

### Phase 3：Worker Node Registry 与 Remote Provider

- 增加 `WorkerNodeRegistry` service/provider contract，heartbeat snapshot 包含 capabilities、resource capacity、current reservations、pressure、health、software/runtime versions、supported isolation classes。
- 在 runtime-host 下添加 remote worker transport adapter。
- local provider 保持默认；remote provider 作为可选模块。
- 未配置 remote cluster 时，remote-only placement 返回 unavailable provider behavior。

验证：

- remote disabled 时，remote-only placement 返回结构化 unavailable。
- remote enabled 时，一个任务可在 B 节点执行，另一个任务可在本机执行。
- B 节点丢失后，lease expiry 能进入可恢复状态。

### Phase 4：Placement Strategy Service

- 实现 filter/score/reserve/bind pipeline。
- 增加 strategy config 和安全 reason code。
- 支持 hard constraint、soft affinity、session stickiness、capability routing、node pressure rejection。
- 增加公平性：防止单个 application/session/tenant 占满所有 lane。

验证：

- 资源压力会让新 run 被放到其他节点或进入 queued。
- capability mismatch 返回 `unsupported`，而不是 panic 或假成功。
- placement decision 可追踪、可 replay。

### Phase 5：可选 Orchestrator Provider

- Kubernetes/Nomad/Ray 类 provider 只能作为同一 placement/worker contract 后面的可选 provider。
- base OS 不能依赖这些 provider。
- 外部 provider 只能接收安全 payload ref 和声明资源请求，不能接收 raw prompt、manifest、secret。

验证：

- dependency-boundary tests 证明 orchestrator crates 是 optional module。
- 没有 orchestrator dependency 时，base local execution 仍然工作。

## API 草图

```rust
pub struct StartExecutionRunCommand {
    pub trace: TraceContext,
    pub scope: ExecutionScope,
    pub profile: ExecutionProfile,
    pub payload_ref: AutonomyPayloadRef,
    pub idempotency_key: Option<String>,
}

pub struct ExecutionScope {
    pub tenant_id: Option<String>,
    pub application_id: ApplicationId,
    pub session_id: String,
    pub task_id: Option<TaskId>,
    pub run_id: ExecutionRunId,
}

pub struct ExecutionProfile {
    pub agent_profile_id: Option<String>,
    pub required_capabilities: Vec<CapabilityId>,
    pub resources: ResourceRequest,
    pub isolation: IsolationClass,
    pub priority: u32,
    pub max_attempts: u32,
    pub placement_preferences: Vec<PlacementPreference>,
}
```

精确 DTO 应在 OpenSpec 中确认后再实现。

## 风险与缓解

- tool 重复执行：强制 side-effect receipt 和 idempotency key。
- workspace 冲突：默认 per-run workspace overlay；共享写入必须经过 file lease、merge protocol 或 review gate。
- context 污染：Memory/Context service 调用必须带 session/run scope 与 policy。
- scheduler 过载：先做 bounded local queue 和简单 placement，不要对 token 级事件调度。
- 饥饿问题：增加 per-tenant/app/session fairness 和 max active runs。
- 节点 split brain：lease 必须有 monotonic generation/fencing token；失去 lease 的旧 worker 不能 commit。
- secret 泄露：remote worker 获取 scoped capability handle，而不是 raw credential。
- 成本失控：placement policy 必须考虑 budget 与 rate limit。
- 过度设计：不实现 live process migration，通过 checkpoint/retry 恢复。

## 需要的 OpenSpec 工作

代码变更前需要创建 proposal。可能影响的 specs：

- `execution-control-service`
- `service-runtime`
- `sdk-system-facade`
- `serviceization-dependency-gate`
- `web-cli-thin-shell-v0` 或 `web-cli-thin-shell-completion`

建议新增 requirements：

- 系统 SHALL 在 policy/resource admission 成功时，允许同一 application 存在多个不同 `run_id` 的 active execution runs。
- 系统 SHALL 按 application、session、task、run 标识隔离 execution state。
- 系统 SHALL 在副作用发生前，通过服务化 placement 与 worker lease 边界路由每个 run。
- 系统 SHALL 在 distributed execution unavailable 时仍支持 local-only execution。
- 系统 SHALL 在 worker node heartbeat 丢失或 lease ownership 丢失时，对 leased run 进行恢复或结构化失败。
- 系统 SHALL 向 shell 暴露 sanitized run events 和 replay diagnostics，但 shell 不拥有调度语义。

## 验收门槛

- 同一 application 可启动 N 个 run，不要求用户创建 N 个 application session。
- 同一 application 下 N 个 session 可同时运行。
- 两个 session 中相同 agent type 的任务拥有独立 prompt、tool、task board、event stream、checkpoint、cancellation、budget state。
- 取消/停止一个 run 不会取消无关 run。
- remote worker 缺失时返回结构化 unavailable，本机执行仍然可用。
- worker/node crash 会产生 bounded recovery 和 trace/audit evidence。
- dependency-boundary tests 拒绝 kernel/provider/shell ownership leak。
- log/snapshot 不出现 raw prompt、manifest、package bytes、provider payload、credential 或 unbounded output。

## 推荐结论

增量采用方案 B：

1. 先实现本机多 run `ExecutionRun` 语义。
2. 再加入 durable lease、checkpoint、recovery。
3. 再加入 worker node registry 与 remote provider。
4. 再加入 placement strategies。
5. 等 Macaca 自身的通用 execution contract 稳定后，再加入 Kubernetes/Nomad/Ray 可选 provider。

这条路径复用成熟分布式系统模式，同时保持 Macaca 的 microkernel/serviceized 宪法。它避免两个极端：既不会继续把执行串行化在 session UI lock 后面，也不会在 Agent OS run contract 尚未稳定前，把 Macaca 过早变成完整容器调度器。
