import type { ExecutionEvent, PresentedTimelineEvent, TimelineEntry } from './types';

export function presentTimelineEvent(entry: TimelineEntry): PresentedTimelineEvent {
  const event = entry.type === 'execution_event' ? entry.data as ExecutionEvent : null;
  if (event) return presentExecutionEvent(event);
  if (entry.type === 'stream_connected') {
    const data = entry.data as Record<string, unknown> | null;
    return {
      title: 'Live updates connected',
      body: 'This session is now following backend execution events in real time.',
      meta: data?.since ? [`Resumed after event ${String(data.since)}`] : [],
    };
  }
  if (entry.type === 'execution_start_result') {
    const data = entry.data as Record<string, unknown> | null;
    return {
      title: 'Task accepted',
      body: 'Macaca accepted the task and assigned a backend execution session.',
      meta: compactMeta({ session: data?.session_id, run: data?.run_id, provider: data?.provider_kind }),
    };
  }
  if (entry.type === 'bridge_error') {
    const data = entry.data as Record<string, unknown> | null;
    return { title: 'Connection issue', body: String(data?.error || 'The workbench bridge reported an error.'), meta: [] };
  }
  if (entry.type === 'assistant_response') {
    const data = entry.data as Record<string, unknown> | null;
    const toolCalls = Array.isArray(data?.tool_calls) ? data.tool_calls as Array<{ name?: string }> : [];
    return {
      title: toolCalls.length ? 'Assistant requested tools' : 'Assistant response',
      body: [data?.reasoning_content, data?.content].filter(Boolean).join('\n\n') || 'No assistant text was emitted.',
      meta: compactMeta({ model: data?.model, tools: toolCalls.map((toolCall) => toolCall.name).join(', ') }),
      format: 'markdown',
    };
  }
  if (entry.type === 'tool_result') {
    const data = entry.data as Record<string, unknown> | null;
    const result = isRecord(data?.result) ? data.result : {};
    const display = isRecord(result.display) ? result.display : {};
    return {
      title: String(display.title || `${String(data?.tool || 'Tool')} ${String(data?.status || 'completed')}`),
      body: String(display.body || asRecord(result.output)?.text || JSON.stringify(result, null, 2)),
      meta: compactMeta({ tool: data?.tool, operation: result.operation, file: display.file_path, status: result.status || data?.status }),
      format: display.format === 'json' ? 'json' : 'markdown',
      usePre: display.format === 'json',
    };
  }
  if (entry.type === 'model_catalog') {
    const data = entry.data as Record<string, unknown> | null;
    return { title: 'Model catalog loaded', body: `${data?.providers || 0} providers and ${data?.models || 0} models are available.`, meta: [] };
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
  const title = String(data.display_title || eventTitle(event.event_type));
  const body = String(
    data.display_body
      || data.display_markdown
      || data.content
      || data.output
      || payload.summary
      || 'Execution event received.',
  );
  return {
    title,
    body,
    meta: compactMeta({
      tool: data.tool_name,
      file: data.file_path,
      status: data.status,
      provider: event.provider_kind,
      seq: event.seq,
    }),
    format: data.display_format === 'json' ? 'json' : data.display_markdown ? 'markdown' : 'text',
    usePre: data.display_format === 'json',
  };
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
