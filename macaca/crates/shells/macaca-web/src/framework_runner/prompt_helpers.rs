//! Prompt section and workspace guide loading helpers.

use macaca_host_composition::context::{
    ContextSourceKind, PromptSection, PromptStability, TrustLevel,
};
use macaca_proto::config::WorkspaceGuideSourcesConfig;
use std::path::Path;
pub(crate) fn prompt_section(
    id: impl Into<String>,
    kind: ContextSourceKind,
    stability: PromptStability,
    trust_level: TrustLevel,
    content: impl Into<String>,
) -> PromptSection {
    PromptSection {
        id: id.into(),
        kind,
        stability,
        trust_level,
        content: content.into(),
    }
}

pub(crate) async fn load_workspace_guide_sections(
    app_dir: &Path,
    guides: &WorkspaceGuideSourcesConfig,
) -> Vec<PromptSection> {
    #[derive(Clone)]
    struct EntryRef<'a> {
        priority: i32,
        path: &'a str,
        max_bytes: u32,
    }
    let mut ordered: Vec<EntryRef<'_>> = guides
        .entries
        .iter()
        .map(|e| EntryRef {
            priority: e.priority,
            path: e.relative_path.as_str(),
            max_bytes: e.max_bytes.max(512),
        })
        .collect();
    ordered.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.path.cmp(b.path)));
    let mut sections = Vec::new();
    for (seq, entry) in ordered.into_iter().enumerate() {
        let path = app_dir.join(entry.path);
        let Ok(content) = tokio::fs::read_to_string(&path).await else {
            continue;
        };
        let content = content.trim();
        if content.is_empty() {
            continue;
        }
        let label = entry.path.trim_end_matches(".md").replace('_', " ");
        sections.push(prompt_section(
            format!("200-guide-{seq:03}-{}", entry.path.replace('/', "__")),
            ContextSourceKind::Workspace,
            PromptStability::Stable,
            TrustLevel::Trusted,
            format!(
                "## {label}\n\n{}",
                bounded_guide_content(content, entry.max_bytes as usize)
            ),
        ));
    }
    sections
}

fn bounded_guide_content(content: &str, max_bytes: usize) -> String {
    if content.len() <= max_bytes {
        return content.to_string();
    }
    let excerpt = content
        .chars()
        .scan(0usize, |used, ch| {
            let len = ch.len_utf8();
            if *used + len > max_bytes {
                None
            } else {
                *used += len;
                Some(ch)
            }
        })
        .collect::<String>();
    format!("{excerpt}\n\n[workspace guide truncated at {max_bytes} bytes]")
}
