export type ProviderInfo = {
  provider_id: string;
  healthy?: boolean;
};

export type ModelInfo = {
  provider_id: string;
  model: string;
  display_name?: string;
  available?: boolean;
};

export type ModelRoute = {
  provider_id?: string;
  model?: string;
  source?: string;
};

export type ExecutionEvent = {
  application_id?: string;
  session_id?: string;
  run_id?: string;
  seq?: number;
  timestamp?: string;
  event_type?: string;
  actor?: string;
  provider_kind?: string;
  sanitized_payload?: {
    summary?: string;
    data?: Record<string, unknown> | null;
  };
};

export type TimelineEntry = {
  type: string;
  data: Record<string, unknown> | ExecutionEvent | string | null;
  at: string;
};

export type CurrentExecutionState = {
  lifecycle_state?: string;
  pending_approval_refs?: string[];
};

export type WorkbenchState = {
  running: boolean;
  commandId: string | null;
  sessionId: string | null;
  runId: string | null;
  eventCursor: string | null;
  currentState: CurrentExecutionState | null;
  events: TimelineEntry[];
  result: string;
  providers: ProviderInfo[];
  models: ModelInfo[];
  route: ModelRoute | null;
  tokenSummary: string;
  debugToolLoop: boolean;
};

export type PresentedTimelineEvent = {
  title: string;
  summary?: string;
  body: string;
  meta: string[];
  details?: string[];
  format?: 'markdown' | 'text' | 'json';
  tone?: 'neutral' | 'progress' | 'success' | 'warning' | 'danger';
  usePre?: boolean;
};

export type BridgeResponse = {
  accepted?: boolean;
  result?: { output?: unknown };
  output?: unknown;
  error?: string;
};
