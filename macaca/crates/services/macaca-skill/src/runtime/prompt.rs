//! Prompt assembly with full → compact → truncate strategies.
//!
//! Renders the `<available_skills>` XML block injected into agent system context.
//! Size limits prevent unbounded prompt growth on large skill catalogs.

use macaca_proto::MacacaResult;
use tracing::debug;

use crate::agent_skill::SkillEntry;

use super::config::SkillRuntimeOptions;
use super::projection::project_prompt_entries;

/// Build model-facing prompt text and the entries included after truncation.
pub(crate) async fn build_prompt(
    visible: &[SkillEntry],
    options: &SkillRuntimeOptions,
) -> MacacaResult<(String, Vec<SkillEntry>, bool, bool)> {
    let limits = &options.limits;
    let mut prompt_entries: Vec<SkillEntry> = visible
        .iter()
        .take(limits.max_skills_in_prompt)
        .cloned()
        .collect();
    let mut truncated = visible.len() > prompt_entries.len();
    let mut compact = false;

    if let Some(workspace_dir) = &options.workspace_dir {
        project_prompt_entries(&mut prompt_entries, workspace_dir).await?;
    }

    let mut prompt = format_full_prompt(&prompt_entries);
    if prompt.len() > limits.max_skills_prompt_chars {
        compact = true;
        debug!(
            chars = prompt.len(),
            limit = limits.max_skills_prompt_chars,
            "skill runtime: switching to compact prompt format"
        );
        prompt = format_compact_prompt(&prompt_entries);
    }
    while prompt.len() > limits.max_skills_prompt_chars && !prompt_entries.is_empty() {
        prompt_entries.pop();
        truncated = true;
        prompt = format_compact_prompt(&prompt_entries);
        debug!(
            remaining = prompt_entries.len(),
            chars = prompt.len(),
            "skill runtime: truncating prompt entries for size limit"
        );
    }
    Ok((prompt, prompt_entries, truncated, compact))
}

/// Full XML prompt with name, description, and location per skill.
fn format_full_prompt(entries: &[SkillEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut out = skill_prompt_preamble();
    out.push_str("\n<available_skills>\n");
    for entry in entries {
        out.push_str("  <skill>\n");
        out.push_str(&format!(
            "    <name>{}</name>\n",
            escape_xml(&entry.skill.name)
        ));
        out.push_str(&format!(
            "    <description>{}</description>\n",
            escape_xml(&entry.skill.description)
        ));
        out.push_str(&format!(
            "    <location>{}</location>\n",
            escape_xml(&entry.skill.location.display().to_string())
        ));
        out.push_str("  </skill>\n");
    }
    out.push_str("</available_skills>");
    out
}

/// Compact XML prompt (name + location only) for large catalogs.
fn format_compact_prompt(entries: &[SkillEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut out = skill_prompt_preamble();
    out.push_str("\n<available_skills>\n");
    for entry in entries {
        out.push_str("  <skill>\n");
        out.push_str(&format!(
            "    <name>{}</name>\n",
            escape_xml(&entry.skill.name)
        ));
        out.push_str(&format!(
            "    <location>{}</location>\n",
            escape_xml(&entry.skill.location.display().to_string())
        ));
        out.push_str("  </skill>\n");
    }
    out.push_str("</available_skills>");
    out
}

/// Neutral instructional preamble (not application-specific).
fn skill_prompt_preamble() -> String {
    [
        "The following skills provide specialized instructions for specific tasks.",
        "Use a skill only when the task matches its name or description.",
        "Before applying a skill, read the SKILL.md file at its location.",
        "When a skill references relative files, resolve them against the skill directory.",
        "Do not assume unlisted skills exist.",
        "",
    ]
    .join("\n")
}

/// Escape XML special characters in skill metadata embedded in prompts.
fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
