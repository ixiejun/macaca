declare module '../session_memento.js' {
  export class WorkbenchSessionMementoStore {
    constructor(options?: { fallbackSessionId?: string; storageKey?: string });
    save(state: unknown): void;
    restore(sessionId?: string | null): {
      sessionId: string | null;
      runId: string | null;
      eventCursor: string | null;
      currentState: unknown;
      events: unknown[];
      result: string;
      running: boolean;
      tokenSummary?: string | null;
    };
  }
}

declare module '../loop/controller.js' {
  export class WorkbenchToolLoopController {
    constructor(options: Record<string, unknown>);
    run(input: Record<string, unknown>): Promise<Record<string, unknown>>;
  }
}

declare module '../loop/llm_client.js' {
  export class WorkbenchLlmClient {
    constructor(options: Record<string, unknown>);
  }
}
