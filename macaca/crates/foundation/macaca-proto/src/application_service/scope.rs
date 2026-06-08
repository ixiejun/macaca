//! Application Service scope and policy hint value objects.
//!
//! Scope types encode *where* a command applies (application, session, agent)
//! without carrying runtime handles.  Policy hints carry admission metadata that
//! decorators interpret without embedding application-specific business rules.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ApplicationId, MacacaResult};

use super::validation::non_empty;

/// Explicit scope for Application Service commands.
///
/// The scope is intentionally string/id based.  It can cross a local service
/// bus or future remote transport without carrying runtime handles, registry
/// references, kernel pointers, or presentation-shell state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationServiceScope {
    pub application_id: Option<ApplicationId>,
    pub application_name: Option<String>,
    pub session_id: Option<String>,
    pub agent_name: Option<String>,
}

impl ApplicationServiceScope {
    /// Build application-scoped command metadata.
    pub fn application(application_id: ApplicationId) -> Self {
        Self {
            application_id: Some(application_id),
            application_name: None,
            session_id: None,
            agent_name: None,
        }
    }

    /// Build session-scoped command metadata after validating the session id.
    pub fn session(
        application_id: ApplicationId,
        session_id: impl Into<String>,
    ) -> MacacaResult<Self> {
        Ok(Self {
            application_id: Some(application_id),
            application_name: None,
            session_id: Some(non_empty(
                session_id.into(),
                "application session_id is required",
            )?),
            agent_name: None,
        })
    }
}

/// Policy/admission hints interpreted by runtime decorators or app providers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationServicePolicyHints {
    pub required_permissions: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}
