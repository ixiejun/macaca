## Context

研究报告指出 OpenClaw 的 slot/capability 和 Hermes 的 MemoryProvider 生命周期/setup 体验都值得借鉴。Macaca 需要在不拆额外 crate 的前提下，提供可插拔 provider runtime。

## Goals

- 支持 provider registry 和 profile-based component selection。
- 支持 agent private 与 session shared 分别选择 provider。
- 支持小白用户通过 remote endpoint 或 MCP 替换默认记忆系统。
- 对所有外部 provider 标配 timeout、circuit breaker、diagnostics、secret redaction。
- 保证 provider 失败不终止 agent run。

## Non-Goals

- 不实现所有行业 provider。
- 不引入不受控动态库加载。
- 不让 remote provider 绕过 `MemoryScope`。
- 不把 provider runtime 拆成新 crate。

## Decisions

### Decision 1: Profile drives free composition

配置示例：

```toml
[memory]
profile = "default"

[memory.profiles.default]
agent_private_provider = "builtin"
session_shared_provider = "builtin"
embedding_provider = "dashscope"
vector_backend = "milvus"
active_recall = "hybrid"
knowledge_compiler = "wiki"
```

per-agent/per-session 可以覆盖：

```toml
[memory.agents.coder]
private_provider = "lancedb"

[memory.sessions.default]
shared_provider = "remote-company-rag"
```

### Decision 2: Remote provider is the primary user extension path

小白用户不应该写 Rust。远程 provider 使用 `macaca-memory-v1` HTTP 协议：

- `GET /memory/v1/status`
- `POST /memory/v1/search`
- `POST /memory/v1/get`
- `POST /memory/v1/write`
- `POST /memory/v1/delete`
- `POST /memory/v1/events`

所有请求必须带 `scope`。

### Decision 3: MCP provider is an adapter

MCP adapter 将 standard memory operations 映射到用户配置的 MCP tools：

- search tool
- get tool
- write tool
- delete tool

MCP 输出必须经过 schema validation 和 trust boundary 转换。

### Decision 4: Provider lifecycle is split by capability

不定义巨型 provider trait 强迫所有 provider 实现所有方法。Provider 提供 metadata、status、capabilities；每个 capability 承担具体操作。

可选 lifecycle：

- initialize
- on_turn_start
- on_turn_end
- on_session_switch
- on_pre_compact
- on_delegation
- shutdown

### Decision 5: External calls are wrapped by resilience decorators

所有 remote/MCP provider calls 必须通过：

- timeout
- circuit breaker
- retry policy
- payload size limit
- secret redaction
- diagnostics

## Risks / Trade-offs

- Risk: profile 配置能力强但复杂。
  - Mitigation: setup UI/CLI 只暴露 provider、endpoint、API key；高级配置保留文件。
- Risk: remote provider 返回不可信内容。
  - Mitigation: schema validation、trust marking、dynamic context fencing。
- Risk: provider tools 冲突。
  - Mitigation: registry 检测 tool name conflict，标准工具优先或 namespaced tools。

## Migration Plan

1. 新增 provider registry/profile DTO。
2. 实现 builtin provider factory。
3. 实现 remote provider adapter。
4. 实现 MCP provider adapter。
5. 将 facade/router 接入 profile 选择。
6. 增加 conformance tests。
