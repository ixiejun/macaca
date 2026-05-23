//! Runtime-host semantic review helper for Skill curation.
//!
//! This module keeps optional-provider orchestration out of the main curation
//! handler.  The current slice deliberately uses the Null Object provider; a
//! later factory-wiring slice can replace this helper without changing SDK,
//! shell, or kernel contracts.

use macaca_skill::{
    SkillCurationRunCommand, SkillCurationRunResult, SkillSemanticReviewProvider,
    SkillSemanticReviewRequest, UnavailableSkillSemanticReviewProvider,
};

/// Attach structured unavailable semantic review output to a curation result.
pub(crate) async fn attach_unavailable_semantic_review(
    result: &mut SkillCurationRunResult,
    command: &SkillCurationRunCommand,
    captured_at: chrono::DateTime<chrono::Utc>,
) {
    let semantic_review = UnavailableSkillSemanticReviewProvider
        .review(SkillSemanticReviewRequest {
            trace: command.trace.clone(),
            scope: command.scope.clone(),
            dry_run: command.dry_run,
            recommendations: result.recommendations.clone(),
            captured_at,
        })
        .await;
    result.semantic_analysis_status = semantic_review.status_message();
    result.semantic_review = semantic_review;
}
