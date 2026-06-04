import type { BridgeResponse, WorkbenchState } from './types';

type PendingBridgeCall = {
  resolve: (value: BridgeResponse) => void;
  reject: (error: Error) => void;
  timeout: number;
};

export type BridgeRuntime = {
  hostOrigin: string;
  pending: Map<string, PendingBridgeCall>;
  stateRef: { current: WorkbenchState };
};

export function createBridgeRuntime(stateRef: { current: WorkbenchState }): BridgeRuntime {
  return {
    hostOrigin: resolveHostOrigin(),
    pending: new Map(),
    stateRef,
  };
}

/**
 * Send one provider-neutral bridge command to the host shell.
 *
 * This is the React UI's Adapter over the app-owned iframe bridge.  It does not
 * call backend endpoints directly and it does not know Macaca shell internals;
 * it only posts the typed `macaca.call` envelope already declared in app.yaml.
 */
export function callService(runtime: BridgeRuntime, serviceId: string, operation: string, payload: Record<string, unknown> = {}) {
  return callBridge(runtime, {
    capability: 'service.call',
    service_id: serviceId,
    operation,
    payload,
  });
}

export function callAppExecution(runtime: BridgeRuntime, operation: string, payload: Record<string, unknown>) {
  return callBridge(runtime, {
    capability: 'app.execution',
    operation,
    payload,
  }).then(bridgeOutput);
}

export function callSessionRead(runtime: BridgeRuntime, operation: string, payload: Record<string, unknown>) {
  return callBridge(runtime, {
    capability: 'session.read',
    operation,
    payload,
  }).then(bridgeOutput);
}

export function bridgeOutput(response: BridgeResponse): Record<string, unknown> {
  const output = response?.result?.output ?? response?.output ?? null;
  return typeof output === 'object' && output !== null ? output as Record<string, unknown> : {};
}

export function resolveBridgeMessage(runtime: BridgeRuntime, message: Record<string, unknown>): boolean {
  if (message.type !== 'macaca.result') return false;
  const commandId = typeof message.command_id === 'string' ? message.command_id : '';
  const pending = runtime.pending.get(commandId);
  if (!pending) return true;
  window.clearTimeout(pending.timeout);
  runtime.pending.delete(commandId);
  if (message.ok && (message.response as BridgeResponse | undefined)?.accepted !== false) {
    pending.resolve(message.response as BridgeResponse);
  } else {
    pending.reject(new Error(String(message.error || 'Bridge call failed')));
  }
  return true;
}

function callBridge(runtime: BridgeRuntime, message: Record<string, unknown>): Promise<BridgeResponse> {
  const commandId = crypto.randomUUID();
  const sessionId = runtime.stateRef.current.sessionId;
  return new Promise((resolve, reject) => {
    const timeout = window.setTimeout(() => {
      runtime.pending.delete(commandId);
      reject(new Error(`${String(message.operation || message.capability)} timed out`));
    }, 120000);
    runtime.pending.set(commandId, { resolve, reject, timeout });
    window.parent.postMessage(
      {
        type: 'macaca.call',
        command_id: commandId,
        session_id: sessionId,
        ...message,
      },
      runtime.hostOrigin,
    );
  });
}

function resolveHostOrigin(): string {
  try {
    return document.referrer ? new URL(document.referrer).origin : '*';
  } catch {
    return '*';
  }
}
