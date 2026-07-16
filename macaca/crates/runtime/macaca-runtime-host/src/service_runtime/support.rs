//! Small support helpers for the ServiceRuntime facade.

use std::sync::Arc;

use macaca_proto::{ServiceDescriptor, ServiceHealth, ServiceLifecycleState};

use crate::service_runtime_error::ServiceRuntimeError;

pub(crate) struct RuntimeServiceRecord {
    pub(crate) descriptor: ServiceDescriptor,
    pub(crate) service: Arc<dyn macaca_kernel::SystemService>,
    pub(crate) lifecycle_state: ServiceLifecycleState,
    pub(crate) health: ServiceHealth,
    pub(crate) failure_reason: Option<String>,
}

pub(super) fn lock_error<T>(_: T) -> ServiceRuntimeError {
    ServiceRuntimeError::State("service runtime lock poisoned".into())
}
