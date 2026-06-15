//! Host-owned bootstrap facade for optional service families.
//!
//! Optional Store, Entitlement, Payment, Web3, and EVM services are provider
//! infrastructure. Presentation shells may supply repository handles, but the
//! runtime-host input object and provider lifecycle sequence belong behind this
//! host composition boundary.

use std::sync::Arc;

use macaca_proto::{MacacaResult, PaymentTerms};
use tracing::info;

use crate::persist::{EntitlementStore, EventLog, PaymentStore};
use crate::service_runtime::ServiceRuntime;

/// Summary emitted after optional service bootstrap completes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostOptionalServiceBootstrapReport {
    /// Number of optional service providers started by runtime-host.
    pub started_services: usize,
    /// Startup trace label supplied by the composition root.
    pub trace_label: String,
}

/// Register and start optional host service families through runtime-host.
///
/// Design pattern: **Facade + Abstract Factory**. The shell passes generic
/// repository handles and payment terms; runtime-host still constructs and
/// starts the concrete providers. This keeps optional module absence and
/// lifecycle behavior out of presentation code.
pub async fn bootstrap_host_optional_services(
    service_runtime: Arc<ServiceRuntime>,
    entitlement_store: Arc<dyn EntitlementStore>,
    entitlement_event_log: Option<Arc<EventLog>>,
    payment_store: Arc<dyn PaymentStore>,
    payment_terms: PaymentTerms,
    trace_label: &str,
) -> MacacaResult<HostOptionalServiceBootstrapReport> {
    info!(
        trace_label,
        "host composition bootstrap starting optional services"
    );

    let inputs = crate::service_bootstrap::OptionalServiceBootstrapInputs::new(
        entitlement_store,
        entitlement_event_log,
        payment_store,
        payment_terms,
    );
    let bundle = crate::service_bootstrap::bootstrap_optional_services(
        Arc::clone(&service_runtime),
        inputs,
        trace_label,
    )
    .await?;

    let report = HostOptionalServiceBootstrapReport {
        started_services: bundle.started_services.len(),
        trace_label: trace_label.to_string(),
    };
    info!(
        trace_label = %report.trace_label,
        started_services = report.started_services,
        "host composition bootstrap completed optional services"
    );
    Ok(report)
}
