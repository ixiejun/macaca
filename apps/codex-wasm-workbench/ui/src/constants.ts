export const declaredServices = [
  'service.interaction',
  'service.app_protocol',
  'service.file',
  'service.process',
  'service.sandbox',
  'service.approval',
  'service.hook',
  'service.config',
  'service.code_intelligence',
  'service.git',
  'service.review',
  'service.diagnostics',
  'service.llm',
  'service.tool',
  'service.mcp',
  'service.skill',
];

export const taskTemplates = [
  {
    label: 'Blank task',
    value: '',
  },
  {
    label: 'Workspace inspection',
    value: 'Inspect the current workspace git status, identify changed files, and propose the next safe implementation step.',
  },
  {
    label: 'Capability validation',
    value: 'Run a focused validation of the Codex WASM workbench application, report service availability, and list missing platform capabilities.',
  },
  {
    label: 'Architecture analysis',
    value: 'Analyze the current codebase for the smallest generic change needed to improve this application without adding application-specific OS logic.',
  },
];
