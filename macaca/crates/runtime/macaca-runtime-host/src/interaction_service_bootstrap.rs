//! Runtime-host bootstrap for `service.interaction`.
//!
//! Presentation shells pass explicit persistence and EventLog handles into this
//! helper, but they never construct thread, turn, or item semantics.  The
//! function is an Abstract Factory boundary: it builds the provider-owned store
//! Strategy, registers the provider in the host `ServiceRuntime`, starts it with
//! a trace id, and returns only the stable service id for diagnostics.

use std::sync::Arc;

use macaca_persist::{EventLog, PersistBackend};
use macaca_proto::{KernelServiceId, MacacaError, MacacaResult, TraceContext};
use tracing::info;

use crate::{
    InteractionLedgerStore, InteractionSystemServiceProvider, PersistInteractionLedgerStore,
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

/// Register and start the durable interaction ledger in a host runtime.
///
/// The persistence backend is injected so Web, CLI, tests, and future gateway
/// hosts can choose their storage policy outside this service.  When an
/// `EventLog` is supplied, the provider mirrors sanitized lifecycle events to
/// the existing session replay path, preserving compatibility while the new
/// ledger becomes the source of truth for Thread/Turn/Item records.
pub async fn bootstrap_interaction_service(
    runtime: Arc<ServiceRuntime>,
    persist_backend: Arc<dyn PersistBackend>,
    event_log: Option<Arc<EventLog>>,
    trace_id: impl Into<String>,
) -> MacacaResult<KernelServiceId> {
    let trace = TraceContext::new(trace_id.into());
    let store: Arc<dyn InteractionLedgerStore> =
        Arc::new(PersistInteractionLedgerStore::new(persist_backend));
    let provider = Arc::new(InteractionSystemServiceProvider::with_event_log(
        store, event_log,
    ));
    let descriptor = macaca_proto::workbench::interaction::descriptor().base;
    let service_id = descriptor.id.clone();

    info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "interaction service bootstrap registering provider"
    );
    runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, provider)),
            ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(runtime_error)?;

    info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "interaction service bootstrap starting provider"
    );
    runtime
        .start(&service_id, trace)
        .await
        .map_err(runtime_error)?;
    Ok(service_id)
}

fn runtime_error(error: crate::ServiceRuntimeError) -> MacacaError {
    MacacaError::Config(error.to_string())
}
