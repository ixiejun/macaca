//! Canonical filenames for the **agent profile** context surface.
//!
//! These names are intentionally fixed so every Macaca application can ship the same Markdown
//! contract without OS code branching on application-specific labels.

/// One of the well-known profile files that [`super::ProfileFileContextProvider`] may load.
///
/// The enum doubles as the **Strategy** selector: each variant maps to a deterministic filename,
/// default [`crate::composer::ContextCandidate`] priority, and cache classification. Applications
/// cannot register ad-hoc kinds here — extension happens by adding a new variant with a new file
/// name in the same OS-level contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentProfileFileKind {
    /// Multi-agent coordination / routing guidance.
    Agents,
    /// Voice / values / stylistic tone.
    Soul,
    /// Tool usage rules and guardrails.
    Tools,
    /// Primary identity statement for the agent role.
    Identity,
    /// User-specific preference context.
    User,
    /// Cadence / push-style behavioural reminders (often high churn).
    Heartbeat,
    /// Memory *seed* / policy hints; ingestion into vector stores is **never** automatic here.
    Memory,
}

impl AgentProfileFileKind {
    /// Every supported kind, in deterministic processing order.
    ///
    /// The composer re-sorts by [`crate::composer::ContextCandidate::priority`], so this order is
    /// only relevant for diagnostics locality.
    pub const ALL: &[AgentProfileFileKind] = &[
        Self::Agents,
        Self::Soul,
        Self::Tools,
        Self::Identity,
        Self::User,
        Self::Heartbeat,
        Self::Memory,
    ];

    /// On-disk file name inside the resolved profile root directory.
    #[must_use]
    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Agents => "AGENTS.md",
            Self::Soul => "SOUL.md",
            Self::Tools => "TOOLS.md",
            Self::Identity => "IDENTITY.md",
            Self::User => "USER.md",
            Self::Heartbeat => "HEARTBEAT.md",
            Self::Memory => "MEMORY.md",
        }
    }
}
