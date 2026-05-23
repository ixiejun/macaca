//! Runtime-host semantic review helper for Skill curation.
//!
//! This module keeps optional-provider orchestration out of the main curation
//! handler.  Runtime-host owns the provider factory so SDK, shell, Web, CLI, and
//! kernel contracts remain stable while future semantic providers can be wired
//! behind the same service boundary.

use std::sync::Arc;

use macaca_skill::{
    SkillCurationRunCommand, SkillCurationRunResult, SkillSemanticReviewProvider,
    SkillSemanticReviewRequest, UnavailableSkillSemanticReviewProvider,
};

/// Runtime-host factory for optional semantic review providers.
///
/// The factory is intentionally local to runtime-host.  It applies the Factory
/// pattern without exposing provider construction to SDK or kernel crates, and
/// currently returns the Null Object provider so deterministic curation remains
/// available when no semantic backend is configured.
#[derive(Debug, Default)]
pub(crate) struct RuntimeSkillSemanticReviewProviderFactory;

impl RuntimeSkillSemanticReviewProviderFactory {
    /// Build the semantic review provider for the current runtime profile.
    ///
    /// Future provider integrations can branch here based on runtime-host
    /// configuration.  The current default is auditable and provider-neutral:
    /// it logs the selected unavailable provider and never performs network or
    /// application-specific work.
    pub(crate) fn build(&self) -> Arc<dyn SkillSemanticReviewProvider> {
        tracing::info!(
            provider_id = UnavailableSkillSemanticReviewProvider::PROVIDER_ID,
            "skill semantic review provider factory selected provider"
        );
        Arc::new(UnavailableSkillSemanticReviewProvider)
    }
}

/// Attach structured unavailable semantic review output to a curation result.
pub(crate) async fn attach_unavailable_semantic_review(
    result: &mut SkillCurationRunResult,
    command: &SkillCurationRunCommand,
    captured_at: chrono::DateTime<chrono::Utc>,
) {
    let provider = RuntimeSkillSemanticReviewProviderFactory::default().build();
    let semantic_review = provider
        .review(SkillSemanticReviewRequest {
            trace: command.trace.clone(),
            scope: command.scope.clone(),
            dry_run: command.dry_run,
            recommendations: result.recommendations.clone(),
            similarity_inputs: Vec::new(),
            clustering_inputs: Vec::new(),
            duplicate_inputs: Vec::new(),
            effectiveness_inputs: Vec::new(),
            reference_graph_inputs: Vec::new(),
            budget: Default::default(),
            resource_limits: Default::default(),
            sanitization_policy: Default::default(),
            captured_at,
        })
        .await;
    result.semantic_analysis_status = semantic_review.status_message();
    result.semantic_review = semantic_review;
}

#[cfg(test)]
mod tests {
    use macaca_proto::TraceContext;
    use macaca_skill::{SkillSemanticReviewRequest, SkillSemanticReviewStatus, SkillServiceScope};

    #[tokio::test]
    async fn semantic_review_factory_defaults_to_unavailable_provider() {
        let provider = super::RuntimeSkillSemanticReviewProviderFactory::default().build();
        let result = provider
            .review(SkillSemanticReviewRequest {
                trace: TraceContext::new("trace-semantic-review-factory"),
                scope: SkillServiceScope::default(),
                dry_run: true,
                recommendations: Vec::new(),
                similarity_inputs: Vec::new(),
                clustering_inputs: Vec::new(),
                duplicate_inputs: Vec::new(),
                effectiveness_inputs: Vec::new(),
                reference_graph_inputs: Vec::new(),
                budget: Default::default(),
                resource_limits: Default::default(),
                sanitization_policy: Default::default(),
                captured_at: chrono::Utc::now(),
            })
            .await;

        assert_eq!(result.status, SkillSemanticReviewStatus::Unavailable);
        assert!(result.proposals.is_empty());
        assert!(!result.mutated);
    }
}
