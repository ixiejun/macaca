//! [`ProfileFileContextProvider`]: loads Markdown agent profile files as [`crate::composer::ContextCandidate`] values.
//!
//! Architecture roles:
//! - **Chain of Responsibility**: plugs into [`crate::composer::ContextFacade`] as another
//!   [`crate::composer::ContextProvider`] without mutating raw [`macaca_proto::LlmMessage`] rows.
//! - **Template Method**: file I/O + canonicalisation live in [`super::loader`]; policy stays in
//!   [`super::policy`].
//!
//! Memory safety note (operational, not Rust): `MEMORY.md` is treated exclusively as *seed /
//! audit* text for the composer. This type performs **no** writes to vector stores — that is
//! enforced structurally by omitting any dependency on `macaca-memory`.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::config::AgentProfileContextConfig;
use macaca_proto::MacacaResult;

use crate::composer::{
    ContextCandidate, ContextComposeContext, ContextProvider, ContextProviderDiagnostics,
    ContextProviderOutcome, ContextProviderStage,
};
use crate::estimate::estimate_text_tokens;
use crate::profile::kinds::AgentProfileFileKind;
use crate::profile::loader::{
    canonical_root, load_profile_file_at_canonical_root, ProfileLoadOutput, ProfileSkipReason,
};
use crate::profile::policy::{default_policy_for, ProfileKindPolicy};

/// Filesystem-backed profile provider for the composer pipeline.
#[derive(Debug, Clone)]
pub struct ProfileFileContextProvider {
    /// Directory resolved by the web/runtime shell (persona dir or workspace dir).
    profile_root: PathBuf,
    /// Per-deployment tunables (byte caps, heartbeat / memory toggles, etc.).
    config: AgentProfileContextConfig,
}

impl ProfileFileContextProvider {
    /// Constructs a provider bound to an absolute or normalised directory.
    ///
    /// The directory should already be chosen using [`macaca_proto::config::AgentProfileRootKind`];
    /// this constructor stays unaware of **how** that path was produced.
    #[must_use]
    pub fn new(profile_root: PathBuf, config: AgentProfileContextConfig) -> Self {
        Self {
            profile_root,
            config,
        }
    }

    fn source_id(kind: AgentProfileFileKind) -> String {
        format!("profile_file/{}", kind.file_name())
    }

    fn outcome_for_load(
        kind: AgentProfileFileKind,
        out: ProfileLoadOutput,
        pol: &ProfileKindPolicy,
    ) -> ContextProviderOutcome {
        let mut diagnostics: Vec<ContextProviderDiagnostics> = Vec::new();
        let mut notes: Vec<String> = pol
            .static_diagnostics
            .iter()
            .map(ToString::to_string)
            .collect();
        notes.extend(out.load_notes.iter().cloned());

        if out.truncated_by_cap {
            diagnostics.push(ContextProviderDiagnostics {
                provider_id: "profile_file".into(),
                message: format!(
                    "{} exceeded max_file_bytes; truncated at {} bytes",
                    kind.file_name(),
                    out.raw_bytes
                ),
            });
            notes.push(format!("truncated_by_cap_bytes={}", out.raw_bytes));
        }

        if out.utf8_lossy {
            diagnostics.push(ContextProviderDiagnostics {
                provider_id: "profile_file".into(),
                message: format!("{} used UTF-8 lossy decode", kind.file_name()),
            });
            notes.push("utf8_lossy_decode=true".into());
        }

        let body = out.text.trim();
        if body.is_empty() {
            return ContextProviderOutcome {
                candidates: Vec::new(),
                diagnostics,
                active_recall_report: None,
            };
        }

        let token_estimate = estimate_text_tokens(body).max(1);
        let candidate = ContextCandidate {
            source_id: Self::source_id(kind),
            kind: pol.candidate_kind,
            scope: pol.scope,
            priority: pol.priority,
            trust: pol.trust,
            cache_class: pol.cache_class,
            target: pol.target,
            content: body.to_string(),
            token_estimate,
            diagnostics: notes,
            evidence_memory_ids: Vec::new(),
            digest_strength: None,
            source_report: None,
        };

        ContextProviderOutcome {
            candidates: vec![candidate],
            diagnostics,
            active_recall_report: None,
        }
    }
}

#[async_trait]
impl ContextProvider for ProfileFileContextProvider {
    fn provider_id(&self) -> &str {
        "profile_files"
    }

    fn stage(&self) -> ContextProviderStage {
        ContextProviderStage::StableProfile
    }

    async fn contribute(
        &self,
        _ctx: &ContextComposeContext<'_>,
    ) -> MacacaResult<ContextProviderOutcome> {
        let _ = _ctx; // Reserved for future session-scoped heartbeat gating.

        if !self.config.enabled {
            return Ok(ContextProviderOutcome::default());
        }

        let resolved_root = match canonical_root(&self.profile_root) {
            Ok(p) => p,
            Err(reason) => {
                return Ok(ContextProviderOutcome {
                    candidates: Vec::new(),
                    diagnostics: vec![ContextProviderDiagnostics {
                        provider_id: self.provider_id().to_string(),
                        message: reason.to_string(),
                    }],
                    active_recall_report: None,
                });
            }
        };

        let mut collected: Vec<ContextCandidate> = Vec::new();
        let mut pooled_diagnostics: Vec<ContextProviderDiagnostics> = Vec::new();

        for &kind in AgentProfileFileKind::ALL {
            let skip = match kind {
                AgentProfileFileKind::Heartbeat => !self.config.inject_heartbeat,
                AgentProfileFileKind::Memory => !self.config.include_memory_seed,
                _ => false,
            };

            let policy = default_policy_for(kind, &self.config);
            match load_profile_file_at_canonical_root(&resolved_root, kind, &self.config, skip)
                .await
            {
                Ok(Some(out)) => {
                    let partial = Self::outcome_for_load(kind, out, &policy);
                    collected.extend(partial.candidates);
                    pooled_diagnostics.extend(partial.diagnostics);
                }
                Ok(None) => {}
                Err(ProfileSkipReason::PathNotConfinedToRoot) => {
                    pooled_diagnostics.push(ContextProviderDiagnostics {
                        provider_id: self.provider_id().to_string(),
                        message: format!(
                            "rejected {}: {}",
                            kind.file_name(),
                            ProfileSkipReason::PathNotConfinedToRoot
                        ),
                    });
                }
                Err(ProfileSkipReason::ContentRejected(reason)) => {
                    pooled_diagnostics.push(ContextProviderDiagnostics {
                        provider_id: self.provider_id().to_string(),
                        message: format!("rejected {}: {reason}", kind.file_name()),
                    });
                }
                Err(other) => {
                    pooled_diagnostics.push(ContextProviderDiagnostics {
                        provider_id: self.provider_id().to_string(),
                        message: format!("skipped {}: {other}", kind.file_name()),
                    });
                }
            }
        }

        Ok(ContextProviderOutcome {
            candidates: collected,
            diagnostics: pooled_diagnostics,
            active_recall_report: None,
        })
    }
}

/// Type-erased wrapper for [`ProfileFileContextProvider`] registration sites.
#[must_use]
pub fn profile_provider_arc(
    profile_root: PathBuf,
    config: AgentProfileContextConfig,
) -> Arc<dyn ContextProvider> {
    Arc::new(ProfileFileContextProvider::new(profile_root, config))
}
