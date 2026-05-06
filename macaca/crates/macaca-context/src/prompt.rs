//! Typed prompt section composition.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::estimate::estimate_text_tokens;
use crate::report::{ContextSourceKind, ContextSourceReport};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PromptStability {
    Stable,
    Dynamic,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    Trusted,
    Untrusted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptSection {
    pub id: String,
    pub kind: ContextSourceKind,
    pub stability: PromptStability,
    pub trust_level: TrustLevel,
    pub content: String,
}

impl PromptSection {
    pub fn builder(id: impl Into<String>, kind: ContextSourceKind) -> PromptSectionBuilder {
        PromptSectionBuilder {
            section: Self {
                id: id.into(),
                kind,
                stability: PromptStability::Dynamic,
                trust_level: TrustLevel::Untrusted,
                content: String::new(),
            },
        }
    }

    pub fn estimated_tokens(&self) -> u32 {
        estimate_text_tokens(&self.content)
    }

    pub fn to_report(&self) -> ContextSourceReport {
        ContextSourceReport::included(
            self.id.clone(),
            self.kind.clone(),
            self.id.clone(),
            self.estimated_tokens(),
            self.content.len(),
        )
    }
}

#[derive(Debug, Clone)]
pub struct PromptSectionBuilder {
    section: PromptSection,
}

impl PromptSectionBuilder {
    pub fn stable(mut self) -> Self {
        self.section.stability = PromptStability::Stable;
        self
    }

    pub fn dynamic(mut self) -> Self {
        self.section.stability = PromptStability::Dynamic;
        self
    }

    pub fn trusted(mut self) -> Self {
        self.section.trust_level = TrustLevel::Trusted;
        self
    }

    pub fn untrusted(mut self) -> Self {
        self.section.trust_level = TrustLevel::Untrusted;
        self
    }

    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.section.content = content.into();
        self
    }

    pub fn build(self) -> PromptSection {
        self.section
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledPrompt {
    pub text: String,
    pub stable_text: String,
    pub stable_hash: String,
    pub full_hash: String,
    pub sections: Vec<PromptSection>,
}

/// [`ContextFacade`] composer output: model-call-ready text plus stable/dynamic SHA-256 (`stable_hash` / `full_hash`).
///
/// Paired with [`crate::composer::ContextPlan`] this matches the integration plan’s “compiled context” bundle.
pub type CompiledContext = CompiledPrompt;

#[derive(Debug, Clone)]
pub struct PromptComposer {
    boundary_marker: String,
    sections: Vec<PromptSection>,
}

impl Default for PromptComposer {
    fn default() -> Self {
        Self {
            boundary_marker: "<!-- MACACA_CONTEXT_DYNAMIC_BOUNDARY -->".to_string(),
            sections: Vec::new(),
        }
    }
}

impl PromptComposer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_boundary_marker(mut self, marker: impl Into<String>) -> Self {
        self.boundary_marker = marker.into();
        self
    }

    pub fn push_section(mut self, section: PromptSection) -> Self {
        self.sections.push(section);
        self
    }

    pub fn compile(mut self) -> CompiledPrompt {
        self.sections.sort_by(|a, b| {
            let stable_order = stability_rank(a.stability).cmp(&stability_rank(b.stability));
            stable_order.then_with(|| a.id.cmp(&b.id))
        });

        let stable_sections = self
            .sections
            .iter()
            .filter(|section| section.stability == PromptStability::Stable)
            .map(render_section)
            .collect::<Vec<_>>();
        let dynamic_sections = self
            .sections
            .iter()
            .filter(|section| section.stability == PromptStability::Dynamic)
            .map(render_section)
            .collect::<Vec<_>>();

        let stable_text = stable_sections.join("\n\n");
        let text = if dynamic_sections.is_empty() {
            stable_text.clone()
        } else if stable_text.is_empty() {
            format!(
                "{}\n\n{}",
                self.boundary_marker,
                dynamic_sections.join("\n\n")
            )
        } else {
            format!(
                "{}\n\n{}\n\n{}",
                stable_text,
                self.boundary_marker,
                dynamic_sections.join("\n\n")
            )
        };

        CompiledPrompt {
            stable_hash: hash_text(&stable_text),
            full_hash: hash_text(&text),
            text,
            stable_text,
            sections: self.sections,
        }
    }
}

fn stability_rank(stability: PromptStability) -> u8 {
    match stability {
        PromptStability::Stable => 0,
        PromptStability::Dynamic => 1,
    }
}

fn render_section(section: &PromptSection) -> String {
    if section.trust_level == TrustLevel::Untrusted {
        format!(
            "<context-section id=\"{}\" trust=\"untrusted\">\n{}\n</context-section>",
            section.id, section.content
        )
    } else {
        section.content.clone()
    }
}

fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::ContextSourceKind;

    #[test]
    fn dynamic_change_does_not_change_stable_hash() {
        let stable = PromptSection::builder("core", ContextSourceKind::SystemPrompt)
            .stable()
            .trusted()
            .content("Core rules")
            .build();
        let dynamic_a = PromptSection::builder("time", ContextSourceKind::DynamicPrompt)
            .dynamic()
            .trusted()
            .content("time=1")
            .build();
        let dynamic_b = PromptSection::builder("time", ContextSourceKind::DynamicPrompt)
            .dynamic()
            .trusted()
            .content("time=2")
            .build();

        let a = PromptComposer::new()
            .push_section(stable.clone())
            .push_section(dynamic_a)
            .compile();
        let b = PromptComposer::new()
            .push_section(stable)
            .push_section(dynamic_b)
            .compile();

        assert_eq!(a.stable_hash, b.stable_hash);
        assert_ne!(a.full_hash, b.full_hash);
    }

    #[test]
    fn renders_sections_deterministically() {
        let b = PromptSection::builder("b", ContextSourceKind::SystemPrompt)
            .stable()
            .trusted()
            .content("B")
            .build();
        let a = PromptSection::builder("a", ContextSourceKind::SystemPrompt)
            .stable()
            .trusted()
            .content("A")
            .build();

        let compiled = PromptComposer::new()
            .push_section(b)
            .push_section(a)
            .compile();

        assert!(compiled.text.starts_with("A\n\nB"));
    }

    #[test]
    fn untrusted_sections_are_fenced() {
        let compiled = PromptComposer::new()
            .push_section(
                PromptSection::builder("memory", ContextSourceKind::Memory)
                    .dynamic()
                    .untrusted()
                    .content("remembered text")
                    .build(),
            )
            .compile();

        assert!(compiled.text.contains("trust=\"untrusted\""));
        assert!(compiled.text.contains("remembered text"));
    }
}
