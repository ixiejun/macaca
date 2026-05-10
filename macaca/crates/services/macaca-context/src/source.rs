//! Source rendering and non-destructive pruning contracts.

use serde::{Deserialize, Serialize};

use crate::estimate::estimate_text_tokens;
use crate::prompt::TrustLevel;
use crate::report::{ContextDecisionReport, ContextSourceKind, ContextSourceReport};

/// Stable reference to an original context source.
///
/// Engines and renderers keep this separate from rendered text so reports can
/// point back to the originating artifact even after excerpting or summarizing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextSourceReference {
    pub id: String,
    pub kind: ContextSourceKind,
    pub label: String,
    pub artifact_ref: Option<String>,
}

impl ContextSourceReference {
    /// Create a source reference without an external artifact handle.
    pub fn new(id: impl Into<String>, kind: ContextSourceKind, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind,
            label: label.into(),
            artifact_ref: None,
        }
    }

    /// Attach an optional artifact reference for downstream diagnostics/UI linking.
    pub fn artifact_ref(mut self, artifact_ref: impl Into<String>) -> Self {
        self.artifact_ref = Some(artifact_ref.into());
        self
    }
}

/// Input given to a renderer before pruning/normalization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextRenderInput {
    pub source: ContextSourceReference,
    pub content: String,
    pub trust_level: TrustLevel,
}

impl ContextRenderInput {
    /// Create one render request carrying source metadata, text, and trust level.
    pub fn new(
        source: ContextSourceReference,
        content: impl Into<String>,
        trust_level: TrustLevel,
    ) -> Self {
        Self {
            source,
            content: content.into(),
            trust_level,
        }
    }
}

/// How a source was represented after rendering.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextRenderMode {
    Full,
    Excerpt,
    Summary,
    Dropped,
}

impl ContextRenderMode {
    /// Stable string representation used in reports and persisted events.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Excerpt => "excerpt",
            Self::Summary => "summary",
            Self::Dropped => "dropped",
        }
    }
}

/// Pruning policy output for one render request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PruningDecision {
    pub mode: ContextRenderMode,
    pub max_bytes: usize,
    pub reason: String,
}

impl PruningDecision {
    /// Keep the full source text.
    pub fn full(reason: impl Into<String>) -> Self {
        Self {
            mode: ContextRenderMode::Full,
            max_bytes: usize::MAX,
            reason: reason.into(),
        }
    }

    /// Keep only a bounded excerpt of the source text.
    pub fn excerpt(max_bytes: usize, reason: impl Into<String>) -> Self {
        Self {
            mode: ContextRenderMode::Excerpt,
            max_bytes,
            reason: reason.into(),
        }
    }

    /// Drop the source from prompt text entirely.
    pub fn dropped(reason: impl Into<String>) -> Self {
        Self {
            mode: ContextRenderMode::Dropped,
            max_bytes: 0,
            reason: reason.into(),
        }
    }
}

/// Rendered source plus accounting metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextSnippet {
    pub source: ContextSourceReference,
    pub text: String,
    pub estimated_tokens: u32,
    pub byte_size: usize,
    pub pruned_tokens: u32,
    pub trust_level: TrustLevel,
    pub mode: ContextRenderMode,
}

impl ContextSnippet {
    /// Convert a rendered snippet into its report representation.
    pub fn to_report(&self) -> ContextSourceReport {
        ContextSourceReport::included(
            self.source.id.clone(),
            self.source.kind.clone(),
            self.source.label.clone(),
            self.estimated_tokens,
            self.byte_size,
        )
        .with_rendering(
            self.mode.as_str(),
            trust_level_name(self.trust_level),
            self.source.artifact_ref.clone(),
            self.pruned_tokens,
        )
    }
}

/// Policy trait deciding whether a source should stay full, become an excerpt, or be dropped.
pub trait PruningPolicy: Send + Sync {
    fn decide(&self, input: &ContextRenderInput) -> PruningDecision;
}

/// Default generic pruning thresholds used by the builtin renderer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DefaultPruningPolicy {
    pub max_full_bytes: usize,
    pub max_excerpt_bytes: usize,
    pub drop_empty: bool,
}

impl Default for DefaultPruningPolicy {
    fn default() -> Self {
        Self {
            max_full_bytes: 8 * 1024,
            max_excerpt_bytes: 4 * 1024,
            drop_empty: true,
        }
    }
}

impl PruningPolicy for DefaultPruningPolicy {
    /// Apply size-based, source-agnostic pruning rules.
    fn decide(&self, input: &ContextRenderInput) -> PruningDecision {
        let bytes = input.content.len();
        if self.drop_empty && input.content.trim().is_empty() {
            return PruningDecision::dropped("empty_source");
        }
        if bytes <= self.max_full_bytes {
            return PruningDecision::full("within_full_budget");
        }
        PruningDecision::excerpt(self.max_excerpt_bytes, "source_exceeds_full_budget")
    }
}

/// Optional policy trait for allocating token budget by source kind.
pub trait BudgetPolicy: Send + Sync {
    fn budget_for(&self, kind: &ContextSourceKind, total_budget: u32) -> u32;
}

/// Generic default source-kind budget split.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultBudgetPolicy;

impl BudgetPolicy for DefaultBudgetPolicy {
    /// Allocate budget percentages by broad source category.
    fn budget_for(&self, kind: &ContextSourceKind, total_budget: u32) -> u32 {
        let percent = match kind {
            ContextSourceKind::SystemPrompt | ContextSourceKind::ToolSchema => 30,
            ContextSourceKind::History => 45,
            ContextSourceKind::Memory | ContextSourceKind::WikiDigest => 10,
            ContextSourceKind::Trace
            | ContextSourceKind::ToolResult
            | ContextSourceKind::FileRead
            | ContextSourceKind::CommandOutput
            | ContextSourceKind::SearchResult => 15,
            _ => 10,
        };
        total_budget.saturating_mul(percent) / 100
    }
}

/// Rendering contract for transforming raw context into prompt-safe snippets.
pub trait ContextRenderable: Send + Sync {
    fn render(&self, input: ContextRenderInput) -> ContextSnippet;
}

/// Builtin renderer combining a pruning policy with standardized snippet output.
#[derive(Debug, Clone)]
pub struct DefaultSourceRenderer<P = DefaultPruningPolicy> {
    policy: P,
}

impl Default for DefaultSourceRenderer {
    fn default() -> Self {
        Self {
            policy: DefaultPruningPolicy::default(),
        }
    }
}

impl<P> DefaultSourceRenderer<P> {
    /// Create a renderer with a caller-supplied pruning policy.
    pub fn new(policy: P) -> Self {
        Self { policy }
    }
}

impl<P> ContextRenderable for DefaultSourceRenderer<P>
where
    P: PruningPolicy,
{
    /// Render one source according to the pruning policy and compute accounting metadata.
    fn render(&self, input: ContextRenderInput) -> ContextSnippet {
        let decision = self.policy.decide(&input);
        let original_tokens = estimate_text_tokens(&input.content);
        let text = render_text(&input, &decision);
        let estimated_tokens = estimate_text_tokens(&text);
        ContextSnippet {
            source: input.source,
            byte_size: text.len(),
            text,
            estimated_tokens,
            pruned_tokens: original_tokens.saturating_sub(estimated_tokens),
            trust_level: input.trust_level,
            mode: decision.mode,
        }
    }
}

/// Convert one rendered snippet into a standardized info decision for the report.
pub fn decision_for_snippet(snippet: &ContextSnippet) -> ContextDecisionReport {
    ContextDecisionReport::info(
        format!("context_render_{}", snippet.mode.as_str()),
        format!(
            "Rendered source '{}' as {} (pruned {} estimated tokens).",
            snippet.source.id,
            snippet.mode.as_str(),
            snippet.pruned_tokens
        ),
    )
}

/// Render final prompt text according to the pruning decision.
fn render_text(input: &ContextRenderInput, decision: &PruningDecision) -> String {
    match decision.mode {
        ContextRenderMode::Full => input.content.clone(),
        ContextRenderMode::Dropped => String::new(),
        ContextRenderMode::Summary | ContextRenderMode::Excerpt => {
            if serde_json::from_str::<serde_json::Value>(&input.content).is_ok() {
                return render_json_excerpt(input, decision.max_bytes);
            }
            render_plain_excerpt(input, decision.max_bytes)
        }
    }
}

/// Render a plain-text excerpt with a provenance header.
fn render_plain_excerpt(input: &ContextRenderInput, max_bytes: usize) -> String {
    let excerpt = bounded_chars(&input.content, max_bytes);
    format!(
        "[{} omitted full content; source_id={}; original_bytes={}]\n{}",
        input.source.kind_label(),
        input.source.id,
        input.content.len(),
        excerpt
    )
}

/// Render a JSON-safe excerpt so structured tool output remains valid JSON.
fn render_json_excerpt(input: &ContextRenderInput, max_bytes: usize) -> String {
    serde_json::json!({
        "truncated": true,
        "source_id": input.source.id,
        "source_kind": input.source.kind,
        "original_bytes": input.content.len(),
        "excerpt": bounded_chars(&input.content, max_bytes),
    })
    .to_string()
}

/// Truncate a string by UTF-8 byte budget without cutting a code point in half.
fn bounded_chars(input: &str, max_bytes: usize) -> String {
    input
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
        .collect()
}

/// Stable string name for trust level in reports/events.
fn trust_level_name(level: TrustLevel) -> &'static str {
    match level {
        TrustLevel::Trusted => "trusted",
        TrustLevel::Untrusted => "untrusted",
    }
}

/// Small helper trait for user-friendly labels in excerpt headers.
trait SourceKindLabel {
    fn kind_label(&self) -> &'static str;
}

impl SourceKindLabel for ContextSourceReference {
    fn kind_label(&self) -> &'static str {
        match self.kind {
            ContextSourceKind::ToolResult => "tool result",
            ContextSourceKind::FileRead => "file read",
            ContextSourceKind::CommandOutput => "command output",
            ContextSourceKind::SearchResult => "search result",
            ContextSourceKind::Trace => "trace event",
            _ => "context source",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn large_input(kind: ContextSourceKind, content: String) -> ContextRenderInput {
        ContextRenderInput::new(
            ContextSourceReference::new("source-1", kind, "Source 1").artifact_ref("event/1"),
            content,
            TrustLevel::Untrusted,
        )
    }

    #[test]
    fn large_output_is_rendered_as_bounded_excerpt() {
        let renderer = DefaultSourceRenderer::new(DefaultPruningPolicy {
            max_full_bytes: 10,
            max_excerpt_bytes: 12,
            drop_empty: true,
        });
        let snippet = renderer.render(large_input(
            ContextSourceKind::CommandOutput,
            "abcdefghijklmnopqrstuvwxyz".repeat(100),
        ));

        assert_eq!(snippet.mode, ContextRenderMode::Excerpt);
        assert!(snippet.text.contains("source_id=source-1"));
        assert!(snippet.text.len() < 120);
        assert!(snippet.pruned_tokens > 0);
        assert_eq!(snippet.to_report().source_ref.as_deref(), Some("event/1"));
    }

    #[test]
    fn json_excerpt_remains_valid_json() {
        let renderer = DefaultSourceRenderer::new(DefaultPruningPolicy {
            max_full_bytes: 4,
            max_excerpt_bytes: 16,
            drop_empty: true,
        });
        let snippet = renderer.render(large_input(
            ContextSourceKind::ToolResult,
            serde_json::json!({"stdout": "hello world", "exit_code": 0}).to_string(),
        ));

        serde_json::from_str::<serde_json::Value>(&snippet.text).unwrap();
        assert_eq!(snippet.mode, ContextRenderMode::Excerpt);
    }

    #[test]
    fn default_policy_is_source_kind_based_not_app_specific() {
        let policy = DefaultBudgetPolicy;
        assert_eq!(
            policy.budget_for(&ContextSourceKind::History, 1000),
            policy.budget_for(&ContextSourceKind::History, 1000)
        );
        assert_eq!(policy.budget_for(&ContextSourceKind::Memory, 1000), 100);
    }
}
