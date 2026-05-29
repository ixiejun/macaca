import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { createLoopState, LOOP_STATES, transitionLoopState } from "../loop/state.js";
import { sanitizeToolOutput } from "../loop/sanitize.js";
import {
  createAssistantToolCallMessage,
  createInitialTranscript,
  createToolResultMessage,
} from "../loop/transcript.js";
import { buildToolDefinitions } from "../loop/tool_registry.js";
import { routeToolCall } from "../loop/tool_router.js";

const declaredServices = [
  "service.llm",
  "service.file",
  "service.process",
  "service.sandbox",
  "service.git",
  "service.review",
  "service.diagnostics",
];

test("tool registry exposes only declared and available service tools", () => {
  const tools = buildToolDefinitions({
    declaredServices,
    availableServices: new Set(["service.file", "service.git", "service.llm"]),
  });

  const names = tools.map((tool) => tool.name);
  assert.deepEqual(names.sort(), ["macaca_file", "macaca_git"].sort());
  assert.ok(tools.every((tool) => tool.parameters?.type === "object"));
  const gitTool = tools.find((tool) => tool.name === "macaca_git");
  assert.deepEqual(gitTool.parameters.required, ["operation"]);
  assert.equal(gitTool.parameters.properties.workspace_root, undefined);
});

test("unknown model tool call becomes a structured tool result", async () => {
  const dispatched = [];
  const result = await routeToolCall({
    toolCall: {
      id: "call_unknown",
      name: "write_hello_world_project",
      arguments: { path: "demo" },
    },
    declaredServices,
    availableServices: new Set(["service.file"]),
    callService: async (serviceId, operation, payload) => {
      dispatched.push({ serviceId, operation, payload });
      return { ok: true };
    },
  });

  assert.equal(dispatched.length, 0);
  assert.equal(result.role, "Tool");
  assert.equal(result.tool_call_id, "call_unknown");
  assert.match(result.content, /tool_not_declared/);
});

test("service-level rejected responses become error tool results", async () => {
  const result = await routeToolCall({
    toolCall: {
      id: "call_rejected",
      name: "macaca_git",
      arguments: { operation: "git.status", payload: {} },
    },
    declaredServices,
    availableServices: new Set(["service.git"]),
    callService: async () => ({
      accepted: true,
      result: {
        status: "Ok",
        output: {
          status: "Failed",
          error: "WASM host import payload is invalid",
        },
      },
    }),
  });

  const content = JSON.parse(result.content);
  assert.equal(content.status, "error");
  assert.equal(content.code, "service_result_failed");
});

test("router rejects service calls that miss required generic payload fields", async () => {
  const dispatched = [];
  const result = await routeToolCall({
    toolCall: {
      id: "call_missing_payload",
      name: "macaca_git",
      arguments: { operation: "git.apply_patch", payload: { actor_ref: "tester" } },
    },
    declaredServices,
    availableServices: new Set(["service.git"]),
    callService: async () => {
      dispatched.push("called");
      return { ok: true };
    },
  });

  const content = JSON.parse(result.content);
  assert.equal(dispatched.length, 0);
  assert.equal(content.status, "error");
  assert.equal(content.code, "missing_required_payload_fields");
});

test("router rejects model-supplied workspace roots before service dispatch", async () => {
  const dispatched = [];
  const result = await routeToolCall({
    toolCall: {
      id: "call_host_root",
      name: "macaca_git",
      arguments: {
        operation: "git.status",
        workspace_root: "/Users/quantum/Code/dev",
      },
    },
    declaredServices,
    availableServices: new Set(["service.git"]),
    callService: async () => {
      dispatched.push("called");
      return { accepted: true, result: { status: "Ok", output: { status: "Completed" } } };
    },
  });

  const content = JSON.parse(result.content);
  assert.equal(dispatched.length, 0);
  assert.equal(content.status, "error");
  assert.equal(content.code, "workspace_root_not_model_visible");
});

test("router converts relative file paths into workspace-scoped target payloads", async () => {
  const dispatched = [];
  await routeToolCall({
    toolCall: {
      id: "call_relative_file",
      name: "macaca_file",
      arguments: {
        operation: "file.write",
        path: "frontend/src/App.js",
        content: "export default function App() { return 'Hello'; }",
        create_parent_directories: true,
      },
    },
    declaredServices,
    availableServices: new Set(["service.file"]),
    callService: async (serviceId, operation, payload) => {
      dispatched.push({ serviceId, operation, payload });
      return { accepted: true, result: { status: "Ok", output: { status: "Completed" } } };
    },
  });

  assert.deepEqual(dispatched[0], {
    serviceId: "service.file",
    operation: "file.write",
    payload: {
      target: { path: "frontend/src/App.js", follow_symlinks: false },
      content: "export default function App() { return 'Hello'; }",
      create_parent_directories: true,
      require_approval: false,
      approval_ref: null,
    },
  });
});

test("router supplies complete file DTO defaults for model-visible arguments", async () => {
  const dispatched = [];
  await routeToolCall({
    toolCall: {
      id: "call_file_defaults",
      name: "macaca_file",
      arguments: {
        operation: "file.write",
        path: "package.json",
        content: "{}",
      },
    },
    declaredServices,
    availableServices: new Set(["service.file"]),
    callService: async (serviceId, operation, payload) => {
      dispatched.push({ serviceId, operation, payload });
      return { accepted: true, result: { status: "Ok", output: { status: "Completed" } } };
    },
  });

  assert.deepEqual(dispatched[0].payload, {
    target: { path: "package.json", follow_symlinks: false },
    content: "{}",
    create_parent_directories: true,
    require_approval: false,
    approval_ref: null,
  });
});

test("router accepts model target shorthand for file writes", async () => {
  const dispatched = [];
  await routeToolCall({
    toolCall: {
      id: "call_file_target_shorthand",
      name: "macaca_file",
      arguments: {
        operation: "file.write",
        target: "shared/package.json",
        content: "{}",
      },
    },
    declaredServices,
    availableServices: new Set(["service.file"]),
    callService: async (serviceId, operation, payload) => {
      dispatched.push({ serviceId, operation, payload });
      return { accepted: true, result: { status: "Ok", output: { status: "Completed" } } };
    },
  });

  assert.deepEqual(dispatched[0].payload.target, {
    path: "shared/package.json",
    follow_symlinks: false,
  });
});

test("router rejects absolute and escaping workspace paths", async () => {
  for (const path of ["/tmp/outside.js", "../outside.js", "src/../../outside.js"]) {
    const result = await routeToolCall({
      toolCall: {
        id: `call_${path}`,
        name: "macaca_file",
        arguments: { operation: "file.read", path },
      },
      declaredServices,
      availableServices: new Set(["service.file"]),
      callService: async () => ({ ok: true }),
    });
    assert.equal(JSON.parse(result.content).code, "invalid_workspace_relative_path");
  }
});

test("assistant tool calls and sanitized results are appended to continuation transcript", () => {
  const transcript = createInitialTranscript({
    task: "Create a small frontend and backend sample.",
    route: { provider_id: "test", model: "model-a" },
    tools: [{ name: "macaca_file" }],
  });
  const assistant = createAssistantToolCallMessage({
    content: "",
    reasoning_content: "bounded reasoning metadata",
    tool_calls: [
      {
        id: "call_1",
        name: "macaca_file",
        arguments: { operation: "file.write", payload: { path: "app.js" } },
      },
    ],
  });
  const toolResult = createToolResultMessage("call_1", {
    status: "ok",
    output: sanitizeToolOutput("created\n".repeat(200)),
  });

  transcript.push(assistant, toolResult);

  assert.equal(transcript.at(-2).role, "Assistant");
  assert.equal(transcript.at(-2).tool_calls[0].id, "call_1");
  assert.equal(transcript.at(-1).role, "Tool");
  assert.equal(transcript.at(-1).tool_call_id, "call_1");
  assert.ok(JSON.parse(transcript.at(-1).content).output.truncated);
});

test("loop state transitions reject invalid terminal transitions", () => {
  const state = createLoopState({ maxIterations: 3 });
  transitionLoopState(state, LOOP_STATES.LLM_CALL, { iteration: 1 });
  transitionLoopState(state, LOOP_STATES.COMPLETE, { reason: "final_answer" });

  assert.throws(
    () => transitionLoopState(state, LOOP_STATES.TOOL_DISPATCH),
    /terminal state/,
  );
});

test("workbench source does not contain static Hello World project templates", () => {
  const source = readFileSync(new URL("../app.js", import.meta.url), "utf8");
  assert.doesNotMatch(source, /helloWorldProjectFiles|write_hello_world_project|codex-wasm-hello-world-demo/);
});
