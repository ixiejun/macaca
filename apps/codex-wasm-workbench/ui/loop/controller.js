import { assertLoopCanContinue, createLoopState, LOOP_STATES, transitionLoopState } from "./state.js";
import {
  createAssistantMessage,
  createAssistantToolCallMessage,
  createToolResultMessage,
  createInitialTranscript,
} from "./transcript.js";
import { buildToolDefinitions } from "./tool_registry.js";
import { routeToolCall } from "./tool_router.js";

// The controller owns application-level orchestration: LLM call, model tool
// dispatch, tool-result continuation, and terminal states.  It never performs
// file/process/git work directly; every side effect goes through `routeToolCall`
// and then the Macaca service bridge.
export class WorkbenchToolLoopController {
  constructor({ llmClient, callService, declaredServices, eventSink }) {
    this.llmClient = llmClient;
    this.callService = callService;
    this.declaredServices = declaredServices;
    this.eventSink = eventSink;
  }

  async run({ task, model, route, availableServices }) {
    const loopState = createLoopState();
    const tools = buildToolDefinitions({
      declaredServices: this.declaredServices,
      availableServices,
    });
    const transcript = createInitialTranscript({ task, route, tools });
    const seenToolCalls = new Map();
    let forceFinalAnswer = false;
    this.emit("loop_start", { tools: tools.map((tool) => tool.name), model });

    while (true) {
      const budget = assertLoopCanContinue(loopState);
      if (!budget.ok) {
        transitionLoopState(loopState, LOOP_STATES.FAILED, { reason: budget.reason });
        this.emit("loop_failed", { reason: budget.reason });
        return { status: "failed", reason: budget.reason, transcript };
      }

      transitionLoopState(loopState, LOOP_STATES.LLM_CALL, {
        iteration: loopState.iteration + 1,
      });
      const activeTools = forceFinalAnswer ? [] : tools;
      this.emit("llm_call", { iteration: loopState.iteration, tool_count: activeTools.length });
      const chatResult = await this.llmClient.chat({ messages: transcript, tools: activeTools, model });
      const response = extractLlmResponse(chatResult);
      const toolCalls = response.tool_calls ?? [];

      if (toolCalls.length === 0) {
        transcript.push(createAssistantMessage(response));
        transitionLoopState(loopState, LOOP_STATES.COMPLETE, { reason: "final_answer" });
        this.emit("final_answer", { content: response.content ?? "", model: response.model });
        return { status: "complete", content: response.content ?? "", transcript };
      }

      if (activeTools.length === 0) {
        transcript.push(createAssistantToolCallMessage(response));
        for (const toolCall of toolCalls) {
          transcript.push(createToolResultMessage(toolCall.id, {
            status: "error",
            code: "tools_disabled_final_answer_required",
            metadata: { tool: toolCall.name },
          }));
        }
        transitionLoopState(loopState, LOOP_STATES.FAILED, {
          reason: "model_requested_tool_after_tools_disabled",
        });
        this.emit("loop_failed", { reason: "model_requested_tool_after_tools_disabled" });
        return {
          status: "failed",
          reason: "model_requested_tool_after_tools_disabled",
          transcript,
        };
      }

      transcript.push(createAssistantToolCallMessage(response));
      transitionLoopState(loopState, LOOP_STATES.TOOL_DISPATCH, {
        count: toolCalls.length,
      });
      this.emit("tool_dispatch", {
        count: toolCalls.length,
        tools: toolCalls.map((toolCall) => toolCall.name),
      });

      const batchSignatures = new Set();
      for (const toolCall of toolCalls) {
        loopState.toolCalls += 1;
        const signature = toolCallSignature(toolCall);
        const previousResult = seenToolCalls.get(signature);
        if (previousResult) {
          const repeated = createToolResultMessage(toolCall.id, {
            status: "error",
            code: "repeated_tool_call_blocked",
            metadata: { tool: toolCall.name },
          });
          transcript.push(repeated);
          this.emit("tool_result", {
            tool_call_id: repeated.tool_call_id,
            tool: toolCall.name,
            status: "repeated_tool_call_blocked",
          });
          if (!batchSignatures.has(signature) && shouldForceFinalAfterRepeat(previousResult)) {
            forceFinalAnswer = true;
          }
          continue;
        }
        batchSignatures.add(signature);
        const toolResult = await routeToolCall({
          toolCall,
          declaredServices: this.declaredServices,
          availableServices,
          callService: this.callService,
        });
        seenToolCalls.set(signature, parseToolResult(toolResult.content));
        transcript.push(toolResult);
        this.emit("tool_result", {
          tool_call_id: toolResult.tool_call_id,
          tool: toolCall.name,
          status: safeStatus(toolResult.content),
        });
      }
      transitionLoopState(loopState, LOOP_STATES.TOOL_RESULT, {
        count: toolCalls.length,
      });
      if (forceFinalAnswer) {
        transcript.push({
          role: "System",
          content: "A repeated identical tool call was blocked. Use the existing tool results to produce the final answer without requesting more tools.",
          reasoning_content: null,
          tool_calls: null,
          tool_call_id: null,
        });
      }
    }
  }

  emit(type, data) {
    this.eventSink?.(type, data);
    console.info("[codex-wasm-workbench] loop", type, data);
  }
}

function toolCallSignature(toolCall) {
  return JSON.stringify({
    name: toolCall.name,
    arguments: toolCall.arguments ?? {},
  });
}

function extractLlmResponse(chatResult) {
  const output = chatResult?.result?.output ?? chatResult?.output ?? chatResult;
  return output?.response ?? output ?? {};
}

function safeStatus(content) {
  return parseToolResult(content).status;
}

function parseToolResult(content) {
  try {
    const parsed = JSON.parse(content);
    return { status: parsed.status ?? "unknown", code: parsed.code ?? null };
  } catch {
    return { status: "unknown", code: null };
  }
}

function shouldForceFinalAfterRepeat(result) {
  return ![
    "missing_required_payload_fields",
    "operation_not_allowed",
    "invalid_workspace_relative_path",
    "workspace_root_not_model_visible",
  ].includes(result.code);
}
