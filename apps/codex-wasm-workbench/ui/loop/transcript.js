// The transcript helpers mirror the provider-neutral `LlmMessage` JSON shape
// from `macaca-proto`.  The app builds messages, but model dispatch and
// provider continuation validation remain owned by `service.llm`.
export function createInitialTranscript({ task, route, tools }) {
  return [
    {
      role: "System",
      content: [
        "You are CODEX-WASM-WORKBENCH running inside Macaca OS.",
        "Use only the provided Macaca service tools to inspect, edit, run, and review code.",
        "Do not invent tool results. Do not emit static project templates without tool calls.",
        "Return a concise final answer only after required service-backed work is complete.",
        `Selected route: ${route ? `${route.provider_id}:${route.model}` : "service.llm default"}.`,
        `Visible tools: ${tools.map((tool) => tool.name).join(", ") || "none"}.`,
      ].join("\n"),
      reasoning_content: null,
      tool_calls: null,
      tool_call_id: null,
    },
    {
      role: "User",
      content: task,
      reasoning_content: null,
      tool_calls: null,
      tool_call_id: null,
    },
  ];
}

export function createAssistantToolCallMessage(response) {
  return {
    role: "Assistant",
    content: response.content ?? "",
    reasoning_content: response.reasoning_content ?? null,
    tool_calls: response.tool_calls ?? [],
    tool_call_id: null,
  };
}

export function createAssistantMessage(response) {
  return {
    role: "Assistant",
    content: response.content ?? "",
    reasoning_content: response.reasoning_content ?? null,
    tool_calls: null,
    tool_call_id: null,
  };
}

export function createToolResultMessage(toolCallId, result) {
  return {
    role: "Tool",
    content: JSON.stringify(result),
    reasoning_content: null,
    tool_calls: null,
    tool_call_id: toolCallId,
  };
}
