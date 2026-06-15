//! Value Object: deterministic Skill Creator semantic trigger extraction.

use std::collections::BTreeSet;

use macaca_proto::TaskResult;

use super::types::MAX_SEMANTIC_TRIGGER_PHRASES;

/// Bounded semantic signal used to create Skill Creator aligned proposals.
///
/// The observer cannot author application-specific Skills and must not copy raw
/// task output into generated packages.  This value object applies a small
/// deterministic Specification over sanitized completion text: it keeps
/// reusable Skill OS concepts, folds well-known compound phrases, and emits a
/// trigger-oriented name/procedure that the downstream materialization Builder
/// can turn into valid `SKILL.md` frontmatter.
pub(crate) struct SemanticSkillCreatorSignal {
    pub(crate) target_skill_name: Option<String>,
    pub(crate) trigger_phrases: Vec<String>,
}

impl SemanticSkillCreatorSignal {
    pub(crate) fn from_task_result(result: &TaskResult) -> Self {
        let trigger_phrases = semantic_trigger_phrases(&result.output);
        let target_skill_name = if trigger_phrases.len() >= 2 {
            Some(trigger_phrases.join("-"))
        } else {
            None
        };
        Self {
            target_skill_name,
            trigger_phrases,
        }
    }

    /// Build a Skill Creator-style bounded summary.
    ///
    /// The summary is still count/ref based for audit safety, but when semantic
    /// triggers are available it exposes them as bounded trigger context instead
    /// of leaving materialization to infer identity from generic observer text.
    pub(crate) fn bounded_summary(&self, fallback: String) -> String {
        if self.trigger_phrases.is_empty() {
            return fallback;
        }
        format!(
            "Reusable Skill trigger context: {}; {}",
            self.trigger_phrases.join(", "),
            fallback
        )
    }

    /// Build concise procedural guidance for a generated Skill.
    ///
    /// This mirrors Skill Creator constraints: description/trigger identity
    /// must be meaningful, the body must stay lean, and provenance stays in refs
    /// rather than raw task artifacts.
    pub(crate) fn reusable_procedure(&self) -> String {
        if self.trigger_phrases.is_empty() {
            return "Review linked event-log and Agent Execution trace refs, extract provider-neutral steps that led to verified terminal success, and keep promoted skill drafts governed by curation, approval, rollback, and sanitized evidence gates.".into();
        }
        format!(
            "Use linked event-log and Agent Execution trace refs to repeat tasks involving {}. Keep the generated Skill concise, trigger-oriented, proposal-linked, registry-visible, telemetry-audited, and governed by approval, rollback, and sanitized evidence gates.",
            self.trigger_phrases.join(", ")
        )
    }
}

/// Extract deterministic trigger phrases from bounded completion text.
///
/// The phrase table is generic to the Skill OS domain rather than a particular
/// application.  It preserves compound concepts such as `registry-load-path`
/// because splitting them into individual words makes model-facing Skill names
/// much harder to trigger and audit.
pub(crate) fn semantic_trigger_phrases(output: &str) -> Vec<String> {
    let tokens = normalized_semantic_tokens(output);
    let mut discovered = BTreeSet::new();
    for index in 0..tokens.len() {
        if let Some(phrase) = semantic_compound_phrase(&tokens, index) {
            discovered.insert(phrase);
        }
        if is_semantic_trigger_token(&tokens[index]) {
            discovered.insert(tokens[index].clone());
        }
    }
    if tokens.iter().any(|token| token == "skill") && tokens.iter().any(|token| token == "package")
    {
        discovered.insert("skill-package".into());
    }
    if tokens.iter().any(|token| token == "registry")
        && tokens
            .iter()
            .any(|token| token == "path" || token == "loadpath")
    {
        discovered.insert("registry-load-path".into());
    }
    semantic_trigger_priority_order()
        .iter()
        .filter(|phrase| discovered.contains(**phrase))
        .take(MAX_SEMANTIC_TRIGGER_PHRASES)
        .map(|phrase| (*phrase).to_string())
        .collect()
}

fn normalized_semantic_tokens(output: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for character in output.chars() {
        if character.is_ascii_alphanumeric() {
            current.push(character.to_ascii_lowercase());
        } else if !current.is_empty() {
            push_normalized_token(&mut tokens, &current);
            current.clear();
        }
    }
    if !current.is_empty() {
        push_normalized_token(&mut tokens, &current);
    }
    tokens
}

fn push_normalized_token(tokens: &mut Vec<String>, token: &str) {
    if token.len() < 4 || is_semantic_stop_word(token) {
        return;
    }
    tokens.push(token.to_string());
}

fn semantic_compound_phrase(tokens: &[String], index: usize) -> Option<String> {
    let current = tokens.get(index)?.as_str();
    let next = tokens.get(index + 1).map(String::as_str);
    match (current, next) {
        ("proposal", Some("linked")) => Some("proposal-linked".into()),
        ("skill", Some("package")) => Some("skill-package".into()),
        ("registry", Some("load")) => Some("registry-load-path".into()),
        ("usage", Some("telemetry")) => Some("usage-telemetry".into()),
        ("semantic", Some("naming")) => Some("semantic-naming".into()),
        ("semantic", Some("identity")) => Some("semantic-identity".into()),
        ("follow", Some("optimization")) => Some("follow-up-optimization".into()),
        ("measurable", Some("optimization")) => Some("measurable-optimization".into()),
        _ => None,
    }
}

fn semantic_trigger_priority_order() -> &'static [&'static str] {
    &[
        "materialization",
        "proposal-linked",
        "skill-package",
        "registry-load-path",
        "usage-telemetry",
        "semantic-naming",
        "semantic-identity",
        "measurable-optimization",
        "follow-up-optimization",
        "activation",
        "curation",
        "evaluation",
        "governance",
        "telemetry",
        "validation",
        "optimization",
        "registry",
        "proposal",
    ]
}

fn is_semantic_trigger_token(token: &str) -> bool {
    matches!(
        token,
        "activation"
            | "curation"
            | "evaluation"
            | "governance"
            | "loadpath"
            | "materialization"
            | "optimization"
            | "proposal"
            | "registry"
            | "telemetry"
            | "validation"
            | "verification"
    )
}

fn is_semantic_stop_word(token: &str) -> bool {
    matches!(
        token,
        "actual"
            | "after"
            | "before"
            | "could"
            | "covering"
            | "creation"
            | "future"
            | "measurable"
            | "naming"
            | "reusable"
            | "should"
            | "through"
            | "verification"
            | "wrote"
    )
}
