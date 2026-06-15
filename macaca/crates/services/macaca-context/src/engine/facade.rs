//! Runtime facades over the context engine registry (**Facade** + **Chain of Responsibility**).
//!
//! `ContextRuntimeFacade` applies selection and fallback policy for production call sites.
//! `ContextManagerFacade` wraps a single engine for simpler tests and adapters.

use std::sync::Arc;

use macaca_proto::MacacaResult;
use tracing::{info, warn};

use super::passthrough::PassthroughContextEngine;
use super::registry::ContextEngineRegistry;
use super::types::{
    ContextAfterTurnInput, ContextAssembleInput, ContextAssembleResult, ContextEngine,
    ContextEngineInfo, ContextEngineSelection,
};

/// Runtime-facing facade that applies engine selection and fallback policy.
///
/// **Integration note:** Upper layers (`macaca-runtime`, `macaca-web`, `macaca-framework`)
/// should prefer [`crate::ContextFacade`], which runs the **context composer** pipeline
/// (provider candidates → plan → optional injected user message) before delegating to this
/// engine facade. `ContextRuntimeFacade` remains the thin engine-only wrapper for tests,
/// adapters, and call sites that intentionally skip composer stages.
#[derive(Clone)]
pub struct ContextRuntimeFacade {
    registry: Arc<ContextEngineRegistry>,
    selection: ContextEngineSelection,
}

impl ContextRuntimeFacade {
    /// Build a facade from an explicit registry and selection policy.
    pub fn new(registry: ContextEngineRegistry, selection: ContextEngineSelection) -> Self {
        Self {
            registry: Arc::new(registry),
            selection,
        }
    }

    /// Build a facade backed by the builtin engine set.
    pub fn builtins(selection: ContextEngineSelection) -> Self {
        Self::new(ContextEngineRegistry::with_builtins(), selection)
    }

    /// Build a facade backed by builtin engines plus a caller-supplied overlay registry.
    pub fn builtins_with_overlay(
        selection: ContextEngineSelection,
        overlay: &ContextEngineRegistry,
    ) -> Self {
        Self::new(
            ContextEngineRegistry::with_builtins().merge_from(overlay),
            selection,
        )
    }

    /// Convenience constructor for pure passthrough behavior.
    pub fn passthrough() -> Self {
        Self::builtins(ContextEngineSelection::passthrough_default())
    }

    /// Assemble context using the selected engine and fallback if the primary fails.
    pub async fn assemble(
        &self,
        input: ContextAssembleInput,
    ) -> MacacaResult<ContextAssembleResult> {
        let engine = self
            .registry
            .resolve_or_passthrough(Some(&self.selection.engine_id));
        let requested_primary = self.selection.engine_id.clone();
        match engine.assemble(input.clone()).await {
            Ok(mut result) => {
                result.report.requested_engine_id = requested_primary.clone();
                result.report.engine_fallback_applied = false;
                info!(
                    requested_engine_id = %requested_primary,
                    resolved_engine_id = %result.report.engine_id,
                    message_count = result.messages.len(),
                    "context runtime facade assembled context"
                );
                Ok(result)
            }
            Err(error) => {
                let fallback_id = self.selection.fallback_engine_id.as_str();
                let fallback = self.registry.resolve_or_passthrough(Some(fallback_id));
                warn!(
                    requested_engine_id = %requested_primary,
                    fallback_engine_id = %fallback_id,
                    error = %error,
                    "primary context engine failed; applying fallback"
                );
                let mut result = fallback.assemble(input).await?;
                result.report.requested_engine_id = requested_primary;
                result.report.engine_fallback_applied = true;
                result
                    .report
                    .decisions
                    .push(crate::report::ContextDecisionReport {
                        code: "context_engine_fallback".into(),
                        severity: crate::report::ContextDecisionSeverity::Warning,
                        message: format!(
                            "Context engine '{}' failed and fallback '{}' was used: {}",
                            self.selection.engine_id, fallback_id, error
                        ),
                    });
                info!(
                    resolved_engine_id = %result.report.engine_id,
                    fallback_applied = true,
                    "context runtime facade assembled context via fallback"
                );
                Ok(result)
            }
        }
    }
}

/// Stable metadata for all builtin context engines shipped by this crate.
pub fn list_builtin_engine_infos() -> Vec<ContextEngineInfo> {
    ContextEngineRegistry::with_builtins().list_engine_infos()
}

/// Thin single-engine facade used by simpler call sites and tests.
#[derive(Clone)]
pub struct ContextManagerFacade {
    engine: Arc<dyn ContextEngine>,
}

impl ContextManagerFacade {
    /// Wrap one engine behind a stable facade.
    pub fn new(engine: Arc<dyn ContextEngine>) -> Self {
        Self { engine }
    }

    /// Convenience constructor for the passthrough engine.
    pub fn passthrough() -> Self {
        Self::new(Arc::new(PassthroughContextEngine))
    }

    /// Return metadata for the wrapped engine.
    pub fn engine_info(&self) -> ContextEngineInfo {
        self.engine.info()
    }

    /// Delegate assembly directly to the wrapped engine.
    pub async fn assemble(
        &self,
        input: ContextAssembleInput,
    ) -> MacacaResult<ContextAssembleResult> {
        self.engine.assemble(input).await
    }

    pub async fn after_turn(&self, input: ContextAfterTurnInput) -> MacacaResult<()> {
        self.engine.after_turn(input).await
    }
}
