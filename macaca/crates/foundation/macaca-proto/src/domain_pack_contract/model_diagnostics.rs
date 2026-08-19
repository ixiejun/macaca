use serde::{Deserialize, Serialize};

/// SDK-facing unavailable explanation for one pack declaration.
///
/// Required and optional declarations share this DTO so admission, SDK tooling, and shells can
/// render the same bounded diagnostic while keeping required-pack blocking semantics explicit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackUnavailableDiagnostic {
    pub pack_id: String,
    pub required: bool,
    pub reason_code: String,
    pub message: String,
}

impl DomainPackUnavailableDiagnostic {
    /// Create a diagnostic from already-sanitized pack resolution state.
    pub fn new(
        pack_id: impl Into<String>,
        required: bool,
        reason_code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            pack_id: pack_id.into(),
            required,
            reason_code: reason_code.into(),
            message: message.into(),
        }
    }
}
