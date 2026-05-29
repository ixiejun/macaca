// The LLM client wraps the generic `service.call` bridge with the complete
// `service.llm/llm.chat` command shape.  Web currently hydrates catalog and
// route commands, so the Workbench provides chat scope and trace explicitly
// without changing any Macaca OS service or shell contract.
export class WorkbenchLlmClient {
  constructor({ callService, applicationIdProvider, sessionIdProvider }) {
    this.callService = callService;
    this.applicationIdProvider = applicationIdProvider;
    this.sessionIdProvider = sessionIdProvider;
  }

  async chat({ messages, tools, model }) {
    const trace = createTraceContext("workbench-llm-chat", this.sessionIdProvider());
    const payload = {
      scope: {
        application_id: this.applicationIdProvider(),
        session_id: this.sessionIdProvider() || "workbench-sessionless",
        agent_name: "application-ui",
      },
      trace,
      messages,
      options: {
        model: model || "",
        max_tokens: 4096,
        temperature: 0.2,
        stop_sequences: [],
        tools: tools.length > 0 ? tools : null,
      },
      model_hint: model || null,
      policy: {
        max_prompt_tokens: null,
        max_completion_tokens: 4096,
        max_cost_micros: null,
        privacy_tier: "application-workbench",
        metadata: {},
      },
    };
    return this.callService("service.llm", "llm.chat", payload);
  }
}

function createTraceContext(prefix, sessionId) {
  return {
    trace_id: `${prefix}:${crypto.randomUUID()}`,
    session_id: sessionId || null,
    task_id: null,
    agent: "application-ui",
    emitted_at: new Date().toISOString(),
  };
}
