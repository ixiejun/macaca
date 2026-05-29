import { sanitizeError, sanitizeToolOutput } from "./sanitize.js";
import { createToolResultMessage } from "./transcript.js";
import { findToolDescriptor } from "./tool_registry.js";

// The router is the only place where model tool calls become side effects.  It
// validates the tool name and operation against the generated registry, then
// dispatches through the existing app bridge so policy, trace, audit, and
// service ownership stay outside the application bundle.
export async function routeToolCall({
  toolCall,
  declaredServices,
  availableServices,
  callService,
}) {
  const descriptor = findToolDescriptor(toolCall.name, {
    declaredServices,
    availableServices,
  });
  if (!descriptor) {
    return rejectedToolResult(toolCall.id, "tool_not_declared", {
      tool: toolCall.name,
    });
  }

  const args = normalizeArguments(toolCall.arguments);
  const workspaceScopeError = validateModelWorkspaceScope(args.payload ?? {});
  if (workspaceScopeError) {
    return rejectedToolResult(toolCall.id, workspaceScopeError.code, {
      tool: toolCall.name,
      operation: args.operation,
      path: workspaceScopeError.path,
    });
  }
  if (!descriptor.operations.includes(args.operation)) {
    return rejectedToolResult(toolCall.id, "operation_not_allowed", {
      tool: toolCall.name,
      operation: args.operation,
    });
  }
  const missingFields = missingRequiredPayloadFields(args.operation, args.payload ?? {});
  if (missingFields.length > 0) {
    return rejectedToolResult(toolCall.id, "missing_required_payload_fields", {
      tool: toolCall.name,
      operation: args.operation,
      missing_fields: missingFields,
    });
  }

  try {
    const response = await callService(descriptor.serviceId, args.operation, args.payload ?? {});
    if (!serviceCallSucceeded(response)) {
      return createToolResultMessage(toolCall.id, {
        status: "error",
        code: "service_result_failed",
        service_id: descriptor.serviceId,
        operation: args.operation,
        output: sanitizeToolOutput(response),
      });
    }
    return createToolResultMessage(toolCall.id, {
      status: "ok",
      service_id: descriptor.serviceId,
      operation: args.operation,
      output: sanitizeToolOutput(response),
    });
  } catch (error) {
    return createToolResultMessage(toolCall.id, {
      status: "error",
      code: "service_call_failed",
      service_id: descriptor.serviceId,
      operation: args.operation,
      output: sanitizeError(error),
    });
  }
}

function serviceCallSucceeded(response) {
  if (response?.accepted === false) return false;
  const rawStatus = response?.result?.output?.status ?? response?.result?.status ?? response?.status ?? "ok";
  const status =
    typeof rawStatus === "string"
      ? rawStatus.toLowerCase()
      : Object.keys(rawStatus ?? {})[0]?.toLowerCase() ?? "ok";
  return !["failed", "failure", "rejected", "denied", "error"].includes(status);
}

function missingRequiredPayloadFields(operation, payload) {
  const requirements = {
    "git.apply_patch": ["patch_text", "actor_ref"],
    "file.read": ["target"],
    "file.write": ["target", "content"],
    "file.patch": ["target", "search", "replace"],
    "process.exec": ["executable", "sandbox"],
  };
  return (requirements[operation] ?? []).filter((field) => !hasMeaningfulValue(payload[field]));
}

function hasMeaningfulValue(value) {
  if (value === null || value === undefined) return false;
  if (typeof value === "string") return value.trim().length > 0;
  return true;
}

function normalizeArguments(argumentsValue) {
  if (typeof argumentsValue === "string") {
    try {
      return normalizePayloadEnvelope(JSON.parse(argumentsValue));
    } catch {
      return {};
    }
  }
  return argumentsValue && typeof argumentsValue === "object"
    ? normalizePayloadEnvelope(argumentsValue)
    : {};
}

function normalizePayloadEnvelope(argumentsObject) {
  if (argumentsObject.payload && typeof argumentsObject.payload === "object") {
    return {
      ...argumentsObject,
      payload: normalizeServicePayload(argumentsObject.operation, argumentsObject.payload),
    };
  }
  const { operation, ...payload } = argumentsObject;
  return { operation, payload: normalizeServicePayload(operation, payload) };
}

function normalizeServicePayload(operation, payload) {
  if (typeof operation !== "string" || !operation.startsWith("file.")) return payload;
  if (typeof payload.target === "string") {
    return withFileOperationDefaults(operation, {
      ...payload,
      target: {
        path: payload.target,
        follow_symlinks: payload.follow_symlinks ?? false,
      },
    });
  }
  if (payload.target || !payload.path) return withFileOperationDefaults(operation, payload);
  const { path, follow_symlinks, ...rest } = payload;
  return withFileOperationDefaults(operation, {
    ...rest,
    target: {
      path,
      follow_symlinks: follow_symlinks ?? false,
    },
  });
}

function withFileOperationDefaults(operation, payload) {
  if (operation === "file.read") {
    return {
      ...payload,
      inline_budget_bytes: payload.inline_budget_bytes ?? 65536,
    };
  }
  if (operation === "file.write") {
    return {
      ...payload,
      create_parent_directories: payload.create_parent_directories ?? true,
      require_approval: payload.require_approval ?? false,
      approval_ref: payload.approval_ref ?? null,
    };
  }
  if (operation === "file.patch") {
    return {
      ...payload,
      require_approval: payload.require_approval ?? false,
      approval_ref: payload.approval_ref ?? null,
    };
  }
  return payload;
}

function validateModelWorkspaceScope(payload) {
  if (hasOwn(payload, "workspace_root")) {
    return { code: "workspace_root_not_model_visible" };
  }
  for (const key of ["target", "source", "sandbox"]) {
    const nested = payload[key];
    if (nested && typeof nested === "object" && hasOwn(nested, "workspace_root")) {
      return { code: "workspace_root_not_model_visible" };
    }
  }
  const paths = [];
  if (typeof payload.path === "string") paths.push(payload.path);
  if (typeof payload.target === "string") paths.push(payload.target);
  if (payload.target && typeof payload.target.path === "string") paths.push(payload.target.path);
  if (payload.source && typeof payload.source.path === "string") paths.push(payload.source.path);
  if (payload.sandbox && typeof payload.sandbox.cwd === "string") paths.push(payload.sandbox.cwd);
  const invalidPath = paths.find((path) => !isWorkspaceRelativePath(path));
  return invalidPath ? { code: "invalid_workspace_relative_path", path: invalidPath } : null;
}

function isWorkspaceRelativePath(path) {
  if (!path || path.startsWith("/") || /^[A-Za-z]:[\\/]/.test(path)) return false;
  return !path.split(/[\\/]+/).some((segment) => segment === "..");
}

function hasOwn(value, key) {
  return Object.prototype.hasOwnProperty.call(value, key);
}

function rejectedToolResult(toolCallId, code, metadata) {
  return createToolResultMessage(toolCallId, {
    status: "error",
    code,
    metadata,
  });
}
