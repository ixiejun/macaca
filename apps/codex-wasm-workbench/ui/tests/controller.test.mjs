import assert from "node:assert/strict";
import { test } from "node:test";

import { WorkbenchToolLoopController } from "../loop/controller.js";

const declaredServices = ["service.llm", "service.git"];

test("controller blocks repeated identical tool calls and asks for a final answer", async () => {
  const chatTools = [];
  const llmClient = {
    async chat({ tools }) {
      chatTools.push(tools.map((tool) => tool.name));
      if (tools.length === 0) {
        return { response: { content: "Repository is not clean.", model: "fake", tool_calls: null } };
      }
      return {
        response: {
          content: "",
          model: "fake",
          tool_calls: [
            {
              id: `call_${chatTools.length}`,
              name: "macaca_git",
              arguments: { operation: "git.status", payload: {} },
            },
          ],
        },
      };
    },
  };
  const dispatched = [];
  const controller = new WorkbenchToolLoopController({
    llmClient,
    declaredServices,
    eventSink: () => {},
    callService: async (serviceId, operation, payload) => {
      dispatched.push({ serviceId, operation, payload });
      return { ok: true };
    },
  });

  const result = await controller.run({
    task: "Check git status.",
    model: "fake",
    route: { provider_id: "fake", model: "fake" },
    availableServices: new Set(declaredServices),
  });

  assert.equal(result.status, "complete");
  assert.equal(dispatched.length, 1);
  assert.deepEqual(chatTools.at(-1), []);
});

test("controller fails instead of dispatching tool calls after tools are disabled", async () => {
  const llmClient = {
    async chat() {
      return {
        response: {
          content: "",
          model: "fake",
          tool_calls: [
            {
              id: "call_after_disabled",
              name: "macaca_git",
              arguments: { operation: "git.status", payload: {} },
            },
          ],
        },
      };
    },
  };
  const dispatched = [];
  const controller = new WorkbenchToolLoopController({
    llmClient,
    declaredServices,
    eventSink: () => {},
    callService: async (serviceId, operation, payload) => {
      dispatched.push({ serviceId, operation, payload });
      return { ok: true };
    },
  });

  const result = await controller.run({
    task: "Check git status.",
    model: "fake",
    route: { provider_id: "fake", model: "fake" },
    availableServices: new Set(declaredServices),
  });

  assert.equal(result.status, "failed");
  assert.equal(result.reason, "model_requested_tool_after_tools_disabled");
  assert.equal(dispatched.length, 1);
});

test("controller allows correction after duplicate calls in one assistant turn", async () => {
  let turn = 0;
  const chatTools = [];
  const llmClient = {
    async chat({ tools }) {
      turn += 1;
      chatTools.push(tools.length);
      if (turn === 1) {
        return {
          response: {
            content: "",
            model: "fake",
            tool_calls: [
              { id: "call_a", name: "macaca_git", arguments: { operation: "git.status", payload: {} } },
              { id: "call_b", name: "macaca_git", arguments: { operation: "git.status", payload: {} } },
            ],
          },
        };
      }
      return { response: { content: "Corrected.", model: "fake", tool_calls: null } };
    },
  };
  const controller = new WorkbenchToolLoopController({
    llmClient,
    declaredServices,
    eventSink: () => {},
    callService: async () => ({ ok: true }),
  });

  const result = await controller.run({
    task: "Check git status.",
    model: "fake",
    route: { provider_id: "fake", model: "fake" },
    availableServices: new Set(declaredServices),
  });

  assert.equal(result.status, "complete");
  assert.deepEqual(chatTools, [1, 1]);
});

test("controller keeps tools enabled after repeated validation errors", async () => {
  let turn = 0;
  const chatTools = [];
  const llmClient = {
    async chat({ tools }) {
      turn += 1;
      chatTools.push(tools.length);
      if (turn <= 2) {
        return {
          response: {
            content: "",
            model: "fake",
            tool_calls: [
              { id: `call_bad_${turn}`, name: "macaca_git", arguments: { operation: "git.apply_patch", payload: {} } },
            ],
          },
        };
      }
      return { response: { content: "Validation error corrected.", model: "fake", tool_calls: null } };
    },
  };
  const controller = new WorkbenchToolLoopController({
    llmClient,
    declaredServices,
    eventSink: () => {},
    callService: async () => ({ ok: true }),
  });

  const result = await controller.run({
    task: "Apply a patch.",
    model: "fake",
    route: { provider_id: "fake", model: "fake" },
    availableServices: new Set(declaredServices),
  });

  assert.equal(result.status, "complete");
  assert.deepEqual(chatTools, [1, 1, 1]);
});
