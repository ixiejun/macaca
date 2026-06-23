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
