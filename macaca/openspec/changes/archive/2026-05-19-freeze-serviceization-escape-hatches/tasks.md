## 1. Implementation
- [x] 1.1 Run GitNexus impact for the first symbols touched.
- [x] 1.2 Create the OpenSpec change for serviceization escape-hatch freezing.
- [x] 1.3 Add a static production-source gate for direct runtime/provider reads and hardcoded role names.
- [x] 1.4 Enrich Route C dependency allowlist rows with owner, caller, replacement, expiry, and validation metadata.
- [x] 1.5 Validate OpenSpec and targeted integration gates.

## 2. Follow-Up Tracks
- [x] 2.1 Remove kernel provider compatibility after dependency metadata proves each edge is gone.
  - [x] 2.1.1 Run GitNexus impact for `Kernel`, `KernelBuilder`, `KernelProviderCompat`, and kernel `execute_agent`.
  - [x] 2.1.2 Introduce a provider-neutral agent execution port and move legacy `Agent::run(llm, tools, services)` bridging out of production `Kernel` storage.
  - [x] 2.1.3 Prune the `macaca-kernel -> macaca-tools` and `macaca-kernel -> macaca-task` normal dependency edges after `cargo tree` verification.
  - [x] 2.1.4 Prune the remaining `macaca-kernel -> macaca-persist` normal dependency edge in a separate audit/queue/fork persistence slice.
- [x] 2.2 Move Web toolkit/runtime reads to focused service clients.
  - [x] 2.2.1 Run GitNexus impact for `build_toolkit` and MCP snapshot helpers.
  - [x] 2.2.2 Remove the deprecated Web driver runtime fallback and replace it with structured unavailable diagnostics.
  - [x] 2.2.3 Replace direct Web MCP runtime definition reads with the MCP focused client snapshot command.
  - [x] 2.2.4 Tighten the Web production guard once direct toolkit references are removed.
  - [x] 2.2.5 Validate Web/service checks and Route C/serviceization gates.
- [x] 2.3 Decouple CLI from Web internals and provider construction.
- [x] 2.4 Externalize domain-pack business behavior behind plugin/package providers.
  - [x] 2.4.1 Run GitNexus impact for domain-pack provider symbols.
  - [x] 2.4.2 Replace built-in finance/crypto provider ownership with generic package-provider registration mechanics.
  - [x] 2.4.3 Move finance service ids, live exchange/RSS adapters, and deterministic finance fixtures behind test-only fixture compilation.
  - [x] 2.4.4 Validate optional absence and update task completion.
- [x] 2.5 Make LLM provider/model routing descriptor-driven.
  - [x] 2.5.1 Run GitNexus impact for router and resolver symbols.
  - [x] 2.5.2 Move built-in provider/model prefix rules into descriptor rows consumed by the resolver chain.
  - [x] 2.5.3 Add audit coverage for shell/kernel/provider-name branching.
  - [x] 2.5.4 Validate compatibility and complete task status.
