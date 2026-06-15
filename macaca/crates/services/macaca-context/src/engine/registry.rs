//! Context engine registry (**Registry** pattern).
//!
//! Lightweight id→engine map used by runtime facades. Resolution always guarantees
//! a usable passthrough engine when the requested id is missing (safe default).

use std::collections::HashMap;
use std::sync::Arc;

use tracing::debug;

use crate::source::DefaultSourceRenderer;

use super::passthrough::PassthroughContextEngine;
use super::pruning::PruningContextEngine;
use super::summary::SummaryContextEngine;
use super::types::{ContextEngine, ContextEngineInfo};
use super::windowed::WindowedContextEngine;

/// Registry of available context-engine implementations.
///
/// The registry is intentionally lightweight: callers resolve by id and fall
/// back to passthrough behavior when the requested engine is missing.
#[derive(Default, Clone)]
pub struct ContextEngineRegistry {
    engines: HashMap<String, Arc<dyn ContextEngine>>,
}

impl ContextEngineRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry containing only the passthrough engine.
    pub fn with_passthrough() -> Self {
        Self::new().register(Arc::new(PassthroughContextEngine))
    }

    /// Create a registry with all builtin engines shipped by this crate.
    pub fn with_builtins() -> Self {
        Self::with_passthrough()
            .register(Arc::new(WindowedContextEngine::default()))
            .register(Arc::new(
                PruningContextEngine::<DefaultSourceRenderer>::default(),
            ))
            .register(Arc::new(SummaryContextEngine::default()))
    }

    /// Register one engine under its reported id.
    pub fn register(mut self, engine: Arc<dyn ContextEngine>) -> Self {
        let id = engine.info().id;
        debug!(engine_id = %id, "registered context engine");
        self.engines.insert(id, engine);
        self
    }

    /// Lookup one engine by id.
    pub fn get(&self, id: &str) -> Option<Arc<dyn ContextEngine>> {
        self.engines.get(id).cloned()
    }

    /// Overlay another registry on top of this one, replacing duplicate ids with the overlay copy.
    pub fn merge_from(mut self, overlay: &ContextEngineRegistry) -> Self {
        for (id, engine) in &overlay.engines {
            self.engines.insert(id.clone(), Arc::clone(engine));
        }
        self
    }

    /// Stable list of registered engine ids for diagnostics/UIs.
    pub fn list_engine_ids(&self) -> Vec<String> {
        let mut ids: Vec<_> = self.engines.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Stable list of registered engine metadata for diagnostics/UIs.
    pub fn list_engine_infos(&self) -> Vec<ContextEngineInfo> {
        let mut rows: Vec<_> = self.engines.values().map(|engine| engine.info()).collect();
        rows.sort_by(|a, b| a.id.cmp(&b.id));
        rows
    }

    /// Resolve the requested engine or guarantee a usable passthrough engine.
    pub fn resolve_or_passthrough(&self, id: Option<&str>) -> Arc<dyn ContextEngine> {
        id.and_then(|id| self.get(id))
            .or_else(|| self.get(PassthroughContextEngine::ID))
            .unwrap_or_else(|| Arc::new(PassthroughContextEngine))
    }
}
