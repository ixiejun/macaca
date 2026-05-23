//! Merge eligibility specifications for umbrella skill curation.
//!
//! The types in this module are executable policy specifications, not mutation
//! commands.  They compare sanitized boundary facts for source and target
//! skills so later apply flows can reject unsafe merges before touching
//! lifecycle state, alias maps, support files, or package content.

use serde::{Deserialize, Serialize};

const MAX_MERGE_ELIGIBILITY_ISSUES: usize = 32;

/// Coarse executable semantics used to prevent unsafe knowledge/code merges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillMergeExecutableSemantics {
    KnowledgeOnly,
    SupportFilesOnly,
    Executable,
}

impl Default for SkillMergeExecutableSemantics {
    fn default() -> Self {
        Self::KnowledgeOnly
    }
}

/// Sanitized facts used by the merge eligibility specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMergeEligibilityFacts {
    pub skill_id: String,
    pub source_scope: String,
    pub ownership_class: String,
    pub package_source: String,
    pub permission_set_id: Option<String>,
    pub trust_level: Option<String>,
    pub executable_semantics: SkillMergeExecutableSemantics,
    pub tenant_id: Option<String>,
    pub capability_signature: Vec<String>,
}

impl SkillMergeEligibilityFacts {
    /// Build minimal facts for tests and deterministic curation candidates.
    pub fn new(
        skill_id: impl Into<String>,
        source_scope: impl Into<String>,
        package_source: impl Into<String>,
    ) -> Self {
        Self {
            skill_id: skill_id.into(),
            source_scope: source_scope.into(),
            ownership_class: "unknown".into(),
            package_source: package_source.into(),
            permission_set_id: None,
            trust_level: None,
            executable_semantics: SkillMergeExecutableSemantics::KnowledgeOnly,
            tenant_id: None,
            capability_signature: Vec::new(),
        }
    }
}

/// One bounded reason why a merge candidate set is incompatible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMergeEligibilityIssue {
    pub field: String,
    pub left_skill_id: String,
    pub right_skill_id: String,
    pub message: String,
}

/// Result of evaluating candidate facts against merge eligibility rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMergeEligibilityDecision {
    pub allowed: bool,
    pub issues: Vec<SkillMergeEligibilityIssue>,
}

/// Deterministic merge eligibility policy.
///
/// The default policy requires exact compatibility for scope, ownership,
/// permissions, trust level, package source, executable semantics, tenant, and
/// capability signature.  This conservative policy keeps Macaca generic: a
/// future provider may layer explicit approval overrides, but the baseline
/// service contract never guesses that incompatible skills are safe to merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMergeEligibilityPolicy {
    pub require_scope_match: bool,
    pub require_ownership_match: bool,
    pub require_permission_match: bool,
    pub require_trust_match: bool,
    pub require_package_source_match: bool,
    pub require_executable_semantics_match: bool,
    pub require_tenant_match: bool,
    pub require_capability_match: bool,
}

impl Default for SkillMergeEligibilityPolicy {
    fn default() -> Self {
        Self {
            require_scope_match: true,
            require_ownership_match: true,
            require_permission_match: true,
            require_trust_match: true,
            require_package_source_match: true,
            require_executable_semantics_match: true,
            require_tenant_match: true,
            require_capability_match: true,
        }
    }
}

impl SkillMergeEligibilityPolicy {
    /// Evaluate all candidate facts against the first fact as the target class.
    pub fn evaluate(&self, facts: &[SkillMergeEligibilityFacts]) -> SkillMergeEligibilityDecision {
        let Some(reference) = facts.first() else {
            return SkillMergeEligibilityDecision {
                allowed: false,
                issues: vec![SkillMergeEligibilityIssue {
                    field: "source_skill_ids".into(),
                    left_skill_id: String::new(),
                    right_skill_id: String::new(),
                    message: "merge eligibility requires at least two skill facts".into(),
                }],
            };
        };
        if facts.len() < 2 {
            return SkillMergeEligibilityDecision {
                allowed: false,
                issues: vec![SkillMergeEligibilityIssue {
                    field: "source_skill_ids".into(),
                    left_skill_id: reference.skill_id.clone(),
                    right_skill_id: String::new(),
                    message: "merge eligibility requires source and target skill facts".into(),
                }],
            };
        }

        let mut issues = Vec::new();
        for candidate in facts.iter().skip(1) {
            self.compare_pair(reference, candidate, &mut issues);
            if issues.len() >= MAX_MERGE_ELIGIBILITY_ISSUES {
                break;
            }
        }
        SkillMergeEligibilityDecision {
            allowed: issues.is_empty(),
            issues,
        }
    }

    fn compare_pair(
        &self,
        left: &SkillMergeEligibilityFacts,
        right: &SkillMergeEligibilityFacts,
        issues: &mut Vec<SkillMergeEligibilityIssue>,
    ) {
        self.push_if_mismatch(
            self.require_scope_match,
            "source_scope",
            &left.source_scope,
            &right.source_scope,
            left,
            right,
            issues,
        );
        self.push_if_mismatch(
            self.require_ownership_match,
            "ownership_class",
            &left.ownership_class,
            &right.ownership_class,
            left,
            right,
            issues,
        );
        self.push_if_mismatch(
            self.require_package_source_match,
            "package_source",
            &left.package_source,
            &right.package_source,
            left,
            right,
            issues,
        );
        self.push_if_mismatch(
            self.require_permission_match,
            "permission_set_id",
            &left.permission_set_id,
            &right.permission_set_id,
            left,
            right,
            issues,
        );
        self.push_if_mismatch(
            self.require_trust_match,
            "trust_level",
            &left.trust_level,
            &right.trust_level,
            left,
            right,
            issues,
        );
        self.push_if_mismatch(
            self.require_executable_semantics_match,
            "executable_semantics",
            &left.executable_semantics,
            &right.executable_semantics,
            left,
            right,
            issues,
        );
        self.push_if_mismatch(
            self.require_tenant_match,
            "tenant_id",
            &left.tenant_id,
            &right.tenant_id,
            left,
            right,
            issues,
        );
        self.push_if_mismatch(
            self.require_capability_match,
            "capability_signature",
            &left.capability_signature,
            &right.capability_signature,
            left,
            right,
            issues,
        );
    }

    fn push_if_mismatch<T: PartialEq>(
        &self,
        enabled: bool,
        field: &str,
        left_value: &T,
        right_value: &T,
        left: &SkillMergeEligibilityFacts,
        right: &SkillMergeEligibilityFacts,
        issues: &mut Vec<SkillMergeEligibilityIssue>,
    ) {
        if !enabled || left_value == right_value || issues.len() >= MAX_MERGE_ELIGIBILITY_ISSUES {
            return;
        }
        issues.push(SkillMergeEligibilityIssue {
            field: field.into(),
            left_skill_id: left.skill_id.clone(),
            right_skill_id: right.skill_id.clone(),
            message: format!("merge eligibility requires matching {field}"),
        });
    }
}
