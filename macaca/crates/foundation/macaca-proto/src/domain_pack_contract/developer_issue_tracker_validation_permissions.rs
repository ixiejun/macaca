use crate::AppServiceContractConfig;

/// Reject undeclared issue-tracker permissions before an app can issue service calls.
pub fn validate_developer_issue_tracker_permission_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), String> {
    const ALLOWED: &[&str] = &[
        "issue_tracker.provider.inspect",
        "issue_tracker.project.read",
        "issue_tracker.schema.read",
        "issue_tracker.issue.read",
        "issue_tracker.issue.create",
        "issue_tracker.issue.update",
        "issue_tracker.issue.transition",
        "issue_tracker.comment.read",
        "issue_tracker.comment.write",
        "issue_tracker.label.manage",
        "issue_tracker.assignee.manage",
        "issue_tracker.relation.manage",
        "issue_tracker.attachment.read",
        "issue_tracker.timeline.read",
    ];
    if let Some(scopes) = declaration
        .pack_permission_scopes
        .get("pack.developer.issue.tracker.v1")
    {
        for scope in scopes {
            if !ALLOWED.contains(&scope.as_str()) {
                return Err(format!("unknown issue-tracker permission scope: {scope}"));
            }
        }
    }
    Ok(())
}
