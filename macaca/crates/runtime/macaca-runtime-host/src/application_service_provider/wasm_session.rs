//! WASM session factory and service-policy synchronization for Application Service.
//!
//! **Pattern:** Bridge — maps manifest-declared WASM contracts to host-owned
//! execution sessions and policy overrides without embedding application
//! business logic in the infrastructure layer.

use std::collections::BTreeSet;
use std::sync::Arc;

use macaca_app::expand_service_capabilities;
use macaca_proto::{
    ApplicationId, ServiceResult, TraceContext, WasmExecutionProfile, WasmRuntimeArtifactRef,
    WasmRuntimeSessionRequest,
};

use crate::wasm_runtime_provider::WasmExecutionSession;
use crate::ServicePolicyLayer;

use super::support::service_adapter_error;
use super::ApplicationSystemServiceProvider;

impl ApplicationSystemServiceProvider {
    /// Resolve one app-scoped WASM execution session, creating it lazily.
    ///
    /// The session key is `application_id` and the setup is completely generic:
    /// - ability id comes from sanitized metadata projection (`ability.runtime.wasm` fallback),
    /// - artifact path uses the discovered app install root (`component.wasm`),
    /// - runtime profile is provider default (`wasm_component`).
    ///
    /// This keeps host dispatch app-agnostic and prevents any application-
    /// specific business logic from leaking into the infrastructure layer.
    pub(super) async fn ensure_wasm_session(
        &self,
        app_id: ApplicationId,
        trace: TraceContext,
    ) -> ServiceResult<Option<Arc<dyn WasmExecutionSession>>> {
        let Some(provider) = self.wasm_runtime_provider.as_ref() else {
            return Ok(None);
        };
        if let Some(existing) = self.wasm_sessions.read().await.get(&app_id).cloned() {
            return Ok(Some(existing));
        }

        let registry = self.registry()?;
        let discovered = {
            let guard = registry.read().await;
            guard.get_app(&app_id).cloned()
        };
        let Some(discovered) = discovered else {
            return Ok(None);
        };
        if discovered.manifest.layer != macaca_app::AppLayer::L2Wasm {
            return Ok(None);
        }

        // The manifest v1 YAML adapter synthesizes one runtime ability with a
        // stable id (`ability.runtime.wasm`) for L2 WASM apps. We use that
        // neutral contract id directly so session creation stays deterministic
        // and does not depend on optional metadata-view projection arguments.
        let ability_id = "ability.runtime.wasm".to_string();
        let artifact_path = discovered.path.join("component.wasm");
        let request = WasmRuntimeSessionRequest::new(
            trace.clone(),
            app_id.to_string(),
            ability_id.clone(),
            WasmRuntimeArtifactRef::new(format!("file://{}", artifact_path.display())),
            WasmExecutionProfile::default_wasm_component(),
        )
        .map_err(service_adapter_error)?;
        let session = Arc::from(
            provider
                .create_session(request)
                .await
                .map_err(service_adapter_error)?,
        );
        tracing::info!(
            trace_id = %trace.trace_id,
            app_id = %app_id,
            ability_id = %ability_id,
            artifact = %artifact_path.display(),
            "Created app-scoped WASM execution session for application host dispatch"
        );
        self.wasm_sessions
            .write()
            .await
            .insert(app_id, Arc::clone(&session));
        Ok(Some(session))
    }

    /// Install app-scoped WASM service-call allowlist from manifest contract.
    ///
    /// This is a generic mapping layer:
    /// - input: app `service_contract` declaration (+ domain packs),
    /// - output: policy engine app override (`allow_services`) consumed by host
    ///   import `service.call` router.
    ///
    /// The mapping is app-agnostic and deny-by-default for undeclared services.
    pub(super) async fn sync_wasm_service_policy_for_app(
        &self,
        app_id: &ApplicationId,
    ) -> ServiceResult<()> {
        let Some(engine) = self.wasm_policy_engine.as_ref() else {
            return Ok(());
        };
        let Some(registry) = self.registry.as_ref() else {
            return Ok(());
        };
        let discovered = {
            let guard = registry.read().await;
            guard.get_app(app_id).cloned()
        };
        let Some(discovered) = discovered else {
            tracing::warn!(app_id = %app_id, "Skipping WASM policy sync because app is not discovered");
            return Ok(());
        };
        let expanded = expand_service_capabilities(
            discovered.manifest.service_contract.as_ref(),
            self.domain_pack_catalog.as_ref(),
        );
        engine.set_app_override(
            app_id.to_string(),
            ServicePolicyLayer {
                allow_services: BTreeSet::from_iter(expanded.services.iter().cloned()),
                deny_services: BTreeSet::new(),
                timeout_ms: None,
                max_retries: None,
            },
        );
        tracing::info!(
            app_id = %app_id,
            service_count = expanded.services.len(),
            capabilities_hash = %expanded.capabilities_hash,
            unresolved_pack_count = expanded.unresolved_packs.len(),
            "Synchronized app-scoped WASM service policy from manifest contract"
        );
        Ok(())
    }
}
