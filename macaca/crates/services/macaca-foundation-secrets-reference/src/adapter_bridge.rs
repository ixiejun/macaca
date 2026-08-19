//! Provider Adapter/Bridge boundary for optional secret backends.
//!
//! The bridge carries only a bounded provider-class label and the abstract
//! service factory. Concrete SDK clients, endpoints, credentials, and native
//! request types remain inside the composition root.

use std::sync::Arc;

use crate::{SecretsReferenceProviderFactory, SecretsReferenceService};

/// A replaceable adapter registration with no provider-native surface.
#[derive(Clone)]
pub struct SecretsReferenceAdapterBridge {
    provider_class: String,
    factory: Arc<dyn SecretsReferenceProviderFactory>,
}

impl SecretsReferenceAdapterBridge {
    /// Register an abstract factory under a bounded diagnostic class label.
    pub fn new(
        provider_class: impl Into<String>,
        factory: Arc<dyn SecretsReferenceProviderFactory>,
    ) -> Self {
        Self {
            provider_class: provider_class.into(),
            factory,
        }
    }

    /// Return the safe class label used for selection and audit.
    pub fn provider_class(&self) -> &str {
        &self.provider_class
    }

    /// Construct the provider-neutral service strategy.
    pub fn create(&self) -> Arc<dyn SecretsReferenceService> {
        self.factory.create()
    }
}
