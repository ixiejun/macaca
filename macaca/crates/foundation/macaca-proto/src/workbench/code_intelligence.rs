//! Provider-neutral contracts for `service.code_intelligence`.
//!
//! The service owns generic code search, symbol context, reference discovery,
//! and analyzer diagnostics for any application that needs repository insight.
//! Contracts intentionally expose bounded snippets and provider health metadata
//! instead of raw analyzer payloads so traces and audit records stay safe.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{BoundedSummary, WorkbenchCommand, WorkbenchServiceDescriptor};

pub const SERVICE_ID: &str = "service.code_intelligence";
pub const CAPABILITY_ID: &str = "capability.code_intelligence";
pub const SEARCH_COMMAND: &str = "code.search";
pub const SYMBOL_CONTEXT_COMMAND: &str = "code.symbol_context";
pub const REFERENCES_FIND_COMMAND: &str = "code.references.find";
pub const DIAGNOSTICS_COMMAND: &str = "code.diagnostics";

pub const COMMANDS: &[&str] = &[
    SEARCH_COMMAND,
    SYMBOL_CONTEXT_COMMAND,
    REFERENCES_FIND_COMMAND,
    DIAGNOSTICS_COMMAND,
];

/// Compatibility request used by generic SDK catalogs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub operation: String,
    pub target_ref: Option<String>,
    pub summary: Option<BoundedSummary>,
}

pub type Command = WorkbenchCommand<Request>;

/// Repository-scoped query with explicit budgets for inline evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeSearchRequest {
    pub workspace_root: String,
    pub query: String,
    pub include_globs: Vec<String>,
    pub max_results: usize,
    pub snippet_budget_bytes: usize,
}

/// Symbol lookup request used by analyzers, indexers, or local search adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolContextRequest {
    pub workspace_root: String,
    pub symbol: String,
    pub max_references: usize,
    pub snippet_budget_bytes: usize,
}

/// File reference discovery request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileReferenceRequest {
    pub workspace_root: String,
    pub path: String,
    pub max_results: usize,
}

/// Analyzer diagnostic request.  The analyzer id is provider-neutral metadata;
/// runtime-host composition decides which concrete adapter can satisfy it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyzerDiagnosticsRequest {
    pub workspace_root: String,
    pub analyzer_ref: Option<String>,
    pub changed_paths: Vec<String>,
    pub max_diagnostics: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeLocation {
    pub path: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeSearchResult {
    pub location: CodeLocation,
    pub snippet: BoundedSummary,
    pub score: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolContext {
    pub symbol: String,
    pub definitions: Vec<CodeSearchResult>,
    pub references: Vec<CodeSearchResult>,
    pub provider_health: ProviderHealthDiagnostic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnalyzerSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyzerDiagnostic {
    pub severity: AnalyzerSeverity,
    pub location: CodeLocation,
    pub message: BoundedSummary,
    pub analyzer_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderHealthDiagnostic {
    pub provider_ref: String,
    pub healthy: bool,
    pub generated_at: DateTime<Utc>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodeIntelligenceResponse {
    Search {
        results: Vec<CodeSearchResult>,
        truncated: bool,
        provider_health: ProviderHealthDiagnostic,
    },
    SymbolContext(SymbolContext),
    References {
        path: String,
        results: Vec<CodeSearchResult>,
        truncated: bool,
    },
    Diagnostics {
        diagnostics: Vec<AnalyzerDiagnostic>,
        provider_health: ProviderHealthDiagnostic,
    },
}

pub fn descriptor() -> WorkbenchServiceDescriptor {
    WorkbenchServiceDescriptor::new(
        SERVICE_ID,
        CAPABILITY_ID,
        COMMANDS.to_vec(),
        "service.code_intelligence.events.v1",
        "service.code_intelligence.audit.v1",
        "service.code_intelligence.snapshot.v1",
    )
}
