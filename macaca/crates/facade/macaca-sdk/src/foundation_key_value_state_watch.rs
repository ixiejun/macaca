//! Bounded watch intent for the foundation key-value state SDK Facade.

use macaca_proto::{
    KeyValueWatchNamespaceCommand, MacacaError, MacacaResult, TraceContext,
    FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
};

use super::domain_pack_client::{DomainPackResolveResult, DomainPackServiceCallBuilder};
use super::service_client::ServiceCallCommand;

const MAX_WATCH_EVENTS: u32 = 256;

/// A watch request plus a bounded cancellation intent for the runtime stream owner.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyValueWatchSubscription {
    request: KeyValueWatchNamespaceCommand,
    max_events: u32,
    trace: TraceContext,
}

impl KeyValueWatchSubscription {
    /// Build the stream-start command through the normal service boundary.
    pub fn build(self, resolved: &DomainPackResolveResult) -> MacacaResult<ServiceCallCommand> {
        let Self { request, trace, .. } = self;
        DomainPackServiceCallBuilder::new(
            FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
            "kv.watch_namespace",
            serde_json::to_value(request)?,
            trace,
        )?
        .build(resolved)
    }

    /// Return an opaque cancellation intent for the lifecycle owner to consume.
    pub fn cancellation(&self) -> KeyValueWatchCancellation {
        KeyValueWatchCancellation {
            trace_id: self.trace.trace_id.clone(),
            max_events: self.max_events,
        }
    }
}

/// Bounded watch cancellation metadata; it contains no key, prefix, or provider handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyValueWatchCancellation {
    pub trace_id: String,
    pub max_events: u32,
}

/// Construct a watch subscription whose stream lifecycle remains runtime-owned.
pub fn key_value_watch_subscription(
    request: KeyValueWatchNamespaceCommand,
    max_events: u32,
    trace: TraceContext,
) -> MacacaResult<KeyValueWatchSubscription> {
    if !(1..=MAX_WATCH_EVENTS).contains(&max_events) {
        return Err(MacacaError::Config(format!(
            "key-value watch max_events must be between 1 and {MAX_WATCH_EVENTS}"
        )));
    }
    Ok(KeyValueWatchSubscription {
        request,
        max_events,
        trace,
    })
}
