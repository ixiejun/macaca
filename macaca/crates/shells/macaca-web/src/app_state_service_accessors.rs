//! Provider-neutral service client accessors for [`AppState`].
//!
//! Presentation adapters (routes, framework runners, shell bridges) must not
//! reach into direct LLM field handles on `AppState`. This module
//! implements the **Facade** pattern: a narrow, auditable entry surface that
//! returns focused SDK clients while bootstrap continues to own field wiring.

use std::sync::Arc;

use macaca_sdk::SystemLlmClient;
use tracing::trace;

use crate::state::AppState;

impl AppState {
    /// Returns the serviceized LLM client installed during composition bootstrap.
    ///
    /// Callers use this accessor instead of reading the backing field directly so
    /// escape-hatch scanners can prove shell code crosses `service.llm` through
    /// SDK facades rather than runtime/provider anchors.
    pub fn service_llm_client(&self) -> Arc<dyn SystemLlmClient> {
        trace!("app state exposing service.llm client through facade accessor");
        Arc::clone(&self.llm_client)
    }
}
