//! Deterministic identity and bounded-text Specification helpers.
//!
//! These pure functions implement provider-neutral slug derivation for Skill
//! materialization.  They never call external models, never inspect application
//! names, and always produce the same output for the same sanitized input so
//! audit replay and rollback remain stable across autonomous runs.

/// Extract a short deterministic slug from sanitized proposal evidence.
///
/// Stop words remove generic evidence language, while the remaining tokens
/// preserve task concepts that make model-facing Skill selection possible.
/// At least two distinct tokens are required so single-word noise cannot become
/// a Skill identity.
pub(super) fn semantic_skill_name_from_text(value: &str) -> Option<String> {
    const MAX_TOKENS: usize = 8;
    let mut tokens = Vec::new();
    let mut current = String::new();

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            current.push(character.to_ascii_lowercase());
        } else if !current.is_empty() {
            push_semantic_token(&mut tokens, &current, MAX_TOKENS);
            current.clear();
        }
    }
    if !current.is_empty() {
        push_semantic_token(&mut tokens, &current, MAX_TOKENS);
    }

    if tokens.len() < 2 {
        return None;
    }
    Some(slugify(&tokens.join("-")))
}

/// Push one candidate token when it passes length, stop-word, and dedup gates.
fn push_semantic_token(tokens: &mut Vec<String>, token: &str, max_tokens: usize) {
    if tokens.len() >= max_tokens || token.len() < 4 || is_identity_stop_word(token) {
        return;
    }
    if token.chars().all(|character| character.is_ascii_digit()) {
        return;
    }
    if !tokens.iter().any(|existing| existing == token) {
        tokens.push(token.to_string());
    }
}

/// Generic English stop words that should not dominate Skill identity slugs.
fn is_identity_stop_word(token: &str) -> bool {
    matches!(
        token,
        "this"
            | "that"
            | "when"
            | "with"
            | "from"
            | "into"
            | "future"
            | "task"
            | "tasks"
            | "skill"
            | "skills"
            | "agent"
            | "agents"
            | "verified"
            | "evidence"
            | "record"
            | "records"
            | "bounded"
            | "reusable"
            | "procedure"
            | "procedures"
            | "output"
            | "verify"
            | "inspect"
            | "confirm"
            | "artifact"
            | "artifacts"
            | "service"
            | "owned"
            | "follow"
            | "followup"
    )
}

/// Collapse arbitrary text into a bounded lowercase slug suitable for Skill ids.
pub(super) fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
        if slug.len() >= 80 {
            break;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "materialized-skill".into()
    } else {
        slug
    }
}

/// Normalize whitespace and cap one-line fields such as descriptions.
pub(super) fn bounded_line(value: &str, max_chars: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_chars)
        .collect()
}

/// Trim and cap multi-line blocks such as reusable procedures.
pub(super) fn bounded_block(value: &str, max_chars: usize) -> String {
    value.trim().chars().take(max_chars).collect()
}
