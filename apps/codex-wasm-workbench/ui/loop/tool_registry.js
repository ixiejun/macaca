// The registry is a Strategy table for model-visible tools.  It is generic to
// Macaca service families and deliberately contains no task templates, project
// names, provider names, or application-specific generation logic.
const TOOL_DESCRIPTORS = [
  {
    name: "macaca_file",
    serviceId: "service.file",
    operations: [
      "file.read",
      "file.write",
      "file.patch",
      "file.metadata",
      "file.directory.list",
      "file.diff",
    ],
    description: "Read, write, patch, list, stat, or diff files through service.file. Use workspace-relative paths only; the host bridge injects the application workspace root after policy admission.",
    parameters: {
      type: "object",
      additionalProperties: false,
      required: ["operation"],
      properties: {
        operation: {
          type: "string",
          enum: [
            "file.read",
            "file.write",
            "file.patch",
            "file.metadata",
            "file.directory.list",
            "file.diff",
          ],
        },
        path: {
          type: "string",
          description: "Workspace-relative file path. Preferred for file.read, file.write, file.patch, metadata, list, and diff.",
        },
        target: {
          type: "string",
          description: "Workspace-relative file path shorthand. Use this only when path is not provided.",
        },
        content: {
          type: "string",
          description: "Complete file content for file.write.",
        },
        search: { type: "string" },
        replace: { type: "string" },
        proposed_content: { type: "string" },
        inline_budget_bytes: { type: "integer", minimum: 1 },
        create_parent_directories: { type: "boolean" },
        require_approval: { type: "boolean" },
        approval_ref: { type: ["string", "null"] },
        follow_symlinks: { type: "boolean" },
      },
    },
  },
  {
    name: "macaca_process",
    serviceId: "service.process",
    requiredPeers: ["service.sandbox"],
    operations: ["process.exec", "process.status"],
    description: "Run bounded workspace commands through service.process and service.sandbox. Use a workspace-relative cwd only; the host bridge injects the application workspace root after policy admission.",
  },
  {
    name: "macaca_git",
    serviceId: "service.git",
    operations: ["git.status", "git.diff", "git.apply_patch"],
    description: "Inspect or apply repository changes through service.git in the application workspace. Do not provide workspace_root; the host bridge injects the Macaca-registered application workspace root.",
    parameters: {
      type: "object",
      additionalProperties: false,
      required: ["operation"],
      properties: {
        operation: { type: "string", enum: ["git.status", "git.diff", "git.apply_patch"] },
        pathspecs: { type: "array", items: { type: "string" } },
        inline_budget_bytes: { type: "integer", minimum: 1 },
        patch_text: { type: "string" },
        require_approval: { type: "boolean" },
        approval_ref: { type: ["string", "null"] },
        actor_ref: { type: "string" },
      },
    },
  },
  {
    name: "macaca_review",
    serviceId: "service.review",
    operations: ["review.start", "review.result"],
    description: "Request structured review evidence through service.review.",
  },
  {
    name: "macaca_diagnostics",
    serviceId: "service.diagnostics",
    operations: [
      "diagnostics.snapshot",
      "diagnostics.trace.bundle",
      "diagnostics.health.summary",
    ],
    description: "Read bounded health and trace diagnostics through service.diagnostics.",
  },
  {
    name: "macaca_tool",
    serviceId: "service.tool",
    operations: ["tool.catalog.plan", "tool.invoke", "tool.result.get"],
    description: "Invoke provider-neutral tool service commands through service.tool.",
  },
  {
    name: "macaca_mcp",
    serviceId: "service.mcp",
    optional: true,
    operations: ["mcp.diagnostics.snapshot", "mcp.resource.read"],
    description: "Use optional MCP capabilities when service.mcp is available.",
  },
  {
    name: "macaca_skill",
    serviceId: "service.skill",
    optional: true,
    operations: ["skill.catalog.snapshot", "skill.invoke"],
    description: "Use optional Skill capabilities when service.skill is available.",
  },
];

export function buildToolDefinitions({ declaredServices, availableServices }) {
  return activeDescriptors({ declaredServices, availableServices }).map((descriptor) => ({
    name: descriptor.name,
    description: `${descriptor.description} Allowed operations: ${descriptor.operations.join(", ")}.`,
    parameters: descriptor.parameters ?? {
      type: "object",
      additionalProperties: false,
      required: ["operation", "payload"],
      properties: {
        operation: {
          type: "string",
          enum: descriptor.operations,
        },
        payload: {
          type: "object",
          description: "Typed payload for the selected Macaca service command.",
        },
      },
    },
  }));
}

export function findToolDescriptor(name, context) {
  return activeDescriptors(context).find((descriptor) => descriptor.name === name) ?? null;
}

function activeDescriptors({ declaredServices, availableServices }) {
  const declared = new Set(declaredServices);
  const available = availableServices ?? declared;
  return TOOL_DESCRIPTORS.filter((descriptor) => {
    if (!declared.has(descriptor.serviceId)) return false;
    if (descriptor.requiredPeers?.some((service) => !declared.has(service))) return false;
    return available.has(descriptor.serviceId);
  });
}
