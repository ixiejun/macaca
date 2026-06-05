import type { ExecutionEvent, PresentedTimelineEvent, TimelineEntry } from './types';

export function presentTimelineEvent(entry: TimelineEntry): PresentedTimelineEvent {
  const event = entry.type === 'execution_event' ? entry.data as ExecutionEvent : null;
  if (event) return presentExecutionEvent(event);
  if (entry.type === 'stream_connected') {
    const data = entry.data as Record<string, unknown> | null;
    return {
      title: 'Live updates connected',
      summary: 'The event stream is attached to the backend session.',
      body: 'This session is now following backend execution events in real time.',
      meta: data?.since ? [`Resumed after event ${String(data.since)}`] : [],
      tone: 'progress',
    };
  }
  if (entry.type === 'execution_start_result') {
    const data = entry.data as Record<string, unknown> | null;
    return {
      title: 'Task accepted',
      summary: 'The backend created or reused an execution run for this task.',
      body: 'Macaca accepted the task and assigned a backend execution session.',
      meta: compactMeta({ session: data?.session_id, run: data?.run_id, provider: data?.provider_kind }),
      tone: 'success',
    };
  }
  if (entry.type === 'bridge_error') {
    const data = entry.data as Record<string, unknown> | null;
    return { title: 'Connection issue', summary: 'The app-owned UI could not complete one bridge operation.', body: String(data?.error || 'The workbench bridge reported an error.'), meta: [], tone: 'danger' };
  }
  if (entry.type === 'assistant_response') {
    const data = entry.data as Record<string, unknown> | null;
    const toolCalls = Array.isArray(data?.tool_calls) ? data.tool_calls as Array<{ name?: string }> : [];
    return {
      title: toolCalls.length ? 'Assistant requested tools' : 'Assistant response',
      summary: toolCalls.length ? `The assistant requested ${toolCalls.length} tool call${toolCalls.length === 1 ? '' : 's'}.` : 'The assistant produced a user-visible response.',
      body: [data?.reasoning_content, data?.content].filter(Boolean).join('\n\n') || 'No assistant text was emitted.',
      meta: compactMeta({ model: data?.model, tools: toolCalls.map((toolCall) => toolCall.name).join(', ') }),
      format: 'markdown',
      tone: toolCalls.length ? 'progress' : 'success',
    };
  }
  if (entry.type === 'tool_result') {
    const data = entry.data as Record<string, unknown> | null;
    const result = isRecord(data?.result) ? data.result : {};
    const display = isRecord(result.display) ? result.display : {};
    return {
      title: String(display.title || `${String(data?.tool || 'Tool')} ${String(data?.status || 'completed')}`),
      summary: summarizeToolResult(String(data?.tool || ''), String(data?.status || result.status || 'completed'), display),
      body: String(display.body || asRecord(result.output)?.text || JSON.stringify(result, null, 2)),
      meta: compactMeta({ tool: data?.tool, operation: result.operation, file: display.file_path, status: result.status || data?.status }),
      format: display.format === 'json' ? 'json' : 'markdown',
      tone: String(result.status || data?.status || '').includes('error') ? 'danger' : 'success',
      usePre: display.format === 'json',
    };
  }
  if (entry.type === 'model_catalog') {
    const data = entry.data as Record<string, unknown> | null;
    return { title: 'Model catalog loaded', summary: 'The app-owned UI can now offer model route choices.', body: `${data?.providers || 0} providers and ${data?.models || 0} models are available.`, meta: [], tone: 'success' };
  }
  return {
    title: humanize(entry.type || 'event'),
    body: typeof entry.data === 'string' ? entry.data : JSON.stringify(entry.data || {}, null, 2),
    meta: [],
    usePre: typeof entry.data !== 'string',
  };
}

function presentExecutionEvent(event: ExecutionEvent): PresentedTimelineEvent {
  const payload = event.sanitized_payload || {};
  const data = payload.data || {};
  const title = eventCardTitle(event, data);
  const body = String(
    data.display_body
      || data.display_markdown
      || data.content
      || data.output
      || payload.summary
      || 'Execution event received.',
  );
  const toolName = stringValue(data.tool_name);
  const filePath = stringValue(data.file_path);
  return {
    title,
    summary: eventSummary(event, data, payload.summary),
    body,
    meta: compactMeta({
      stage: data.stage,
      tool: toolName,
      file: filePath ? shortPath(filePath) : '',
      status: data.status,
      actor: event.actor,
      provider: event.provider_kind,
      seq: event.seq,
      time: event.timestamp ? formatTime(event.timestamp) : '',
    }),
    details: eventDetails(event, data),
    format: data.display_format === 'json' ? 'json' : isMarkdownDisplay(data) ? 'markdown' : 'text',
    tone: eventTone(event, data),
    usePre: data.display_format === 'json',
  };
}

function eventCardTitle(event: ExecutionEvent, data: Record<string, unknown>): string {
  const displayTitle = stringValue(data.display_title);
  const toolName = stringValue(data.tool_name);
  const filePath = stringValue(data.file_path);
  if (event.event_type === 'ToolCallRequested' && toolName === 'file_write' && filePath) return `Writing file: ${shortPath(filePath)}`;
  if (event.event_type === 'ToolCallCompleted' && toolName === 'file_write' && filePath) return `File saved: ${shortPath(filePath)}`;
  if (displayTitle) return displayTitle;
  return eventTitle(event.event_type);
}

function eventSummary(event: ExecutionEvent, data: Record<string, unknown>, fallback?: string): string {
  const toolName = stringValue(data.tool_name);
  const filePath = stringValue(data.file_path);
  if (event.event_type === 'LlmRequested') return 'The user task and execution instructions were sent to the coding agent.';
  if (event.event_type === 'ProviderHeartbeat' && stringValue(data.stage) === 'thinking') return `The agent is reasoning through iteration ${String(data.iteration ?? '0')}.`;
  if (event.event_type === 'ToolCallRequested' && toolName) return summarizeToolRequest(toolName, filePath, data);
  if (event.event_type === 'ToolCallCompleted' && toolName) return summarizeToolCompletion(toolName, filePath, data);
  if (event.event_type === 'LlmCompleted') return 'The assistant produced a response for this task.';
  if (event.event_type === 'ExecutionCompleted') return 'The backend marked this execution run as complete.';
  if (event.event_type === 'ExecutionFailed') return 'The backend reported that execution failed.';
  return String(fallback || 'Execution progress was recorded.');
}

function summarizeToolRequest(toolName: string, filePath: string, data: Record<string, unknown>): string {
  if (toolName === 'file_write' && filePath) {
    const bytes = Number(data.content_bytes || extractByteCount(stringValue(data.display_body)));
    return `The agent is preparing to write ${shortPath(filePath)}${bytes ? ` with ${formatBytes(bytes)} of content` : ''}.`;
  }
  return `The agent requested ${toolName}.`;
}

function summarizeToolCompletion(toolName: string, filePath: string, data: Record<string, unknown>): string {
  if (toolName === 'file_write' && filePath) return `The file ${shortPath(filePath)} was written successfully.`;
  if (data.is_error === true) return `${toolName} completed with an error.`;
  return `${toolName} completed successfully.`;
}

function summarizeToolResult(toolName: string, status: string, display: Record<string, unknown>): string {
  const filePath = stringValue(display.file_path);
  if (toolName === 'file_write' && filePath) return `The file ${shortPath(filePath)} was written successfully.`;
  return `${toolName || 'Tool'} reported ${status || 'completed'}.`;
}

function eventDetails(event: ExecutionEvent, data: Record<string, unknown>): string[] {
  return compactMeta({
    event_type: event.event_type,
    session: event.session_id,
    run: event.run_id,
    input_hash: data.input_hash,
    output_hash: data.output_hash,
    input_length: data.input_len,
    output_length: data.output_len,
  });
}

function eventTone(event: ExecutionEvent, data: Record<string, unknown>): PresentedTimelineEvent['tone'] {
  if (event.event_type === 'ExecutionFailed' || data.is_error === true) return 'danger';
  if (event.event_type === 'ExecutionCompleted' || event.event_type === 'ToolCallCompleted' || event.event_type === 'LlmCompleted') return 'success';
  if (event.event_type === 'ToolCallRequested' || event.event_type === 'LlmRequested' || event.event_type === 'ProviderHeartbeat') return 'progress';
  return 'neutral';
}

function eventTitle(eventType?: string): string {
  switch (eventType) {
    case 'LlmRequested': return 'Task sent to the agent';
    case 'LlmCompleted': return 'Assistant response';
    case 'ToolCallRequested': return 'Tool call requested';
    case 'ToolCallCompleted': return 'Tool call completed';
    case 'ExecutionCompleted': return 'Execution completed';
    case 'ExecutionFailed': return 'Execution failed';
    case 'ProviderAssigned': return 'Provider assigned';
    case 'ProviderHeartbeat': return 'Execution progress';
    default: return humanize(eventType || 'execution event');
  }
}

export function compactMeta(values: Record<string, unknown>): string[] {
  return Object.entries(values)
    .filter(([, value]) => value !== null && value !== undefined && value !== '')
    .map(([key, value]) => `${humanize(key)}: ${String(value)}`);
}

export function humanize(value: string): string {
  return String(value)
    .replaceAll('_', ' ')
    .replaceAll('-', ' ')
    .replace(/([a-z])([A-Z])/g, '$1 $2')
    .replace(/\s+/g, ' ')
    .trim()
    .replace(/^./, (char) => char.toUpperCase());
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function asRecord(value: unknown): Record<string, unknown> {
  return isRecord(value) ? value : {};
}

function isMarkdownDisplay(data: Record<string, unknown>): boolean {
  return data.display_format === 'markdown' || typeof data.display_markdown === 'string';
}

function stringValue(value: unknown): string {
  return typeof value === 'string' ? value : '';
}

function shortPath(path: string): string {
  return path.split('/').filter(Boolean).slice(-3).join('/');
}

function extractByteCount(text: string): number {
  const match = text.match(/\((\d+)\s+content bytes\)/);
  return match ? Number(match[1]) : 0;
}

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return '';
  if (bytes < 1024) return `${bytes} B`;
  return `${(bytes / 1024).toFixed(bytes >= 10240 ? 0 : 1)} KB`;
}

function formatTime(timestamp: string): string {
  const date = new Date(timestamp);
  return Number.isNaN(date.getTime()) ? timestamp : date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}
