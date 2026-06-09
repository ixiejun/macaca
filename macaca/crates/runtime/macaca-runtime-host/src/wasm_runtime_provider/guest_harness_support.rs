//! Private helpers for the runtime-owned WASM guest harness.
//!
//! The public harness module intentionally owns the Facade, Proxy, Builder,
//! Adapter, and Test Double API surface.  This support module keeps mechanical
//! fixture construction and sanitization rules separate, so the public module
//! remains small enough to review while every helper stays deterministic,
//! provider-neutral, and reusable across future runtime harness tests.

use macaca_proto::{
    audit_redaction::{sanitize_json_for_harness, sanitize_string_for_audit},
    ApplicationAbilityKind, ApplicationExport, ApplicationHostCommandResult,
    ApplicationHostCommandStatus, ApplicationImport, TraceContext, WasmAbiRequirement,
    WasmArtifactDigest, WasmComponentArtifactDescriptor, WasmExportDeclaration,
    WasmImportRequirement, WasmRuntimeArtifactRef,
};
use serde_json::Value;

use super::guest_harness::WasmExampleFixtureKind;

/// Build the metadata-only component artifact descriptor used by examples.
///
/// The descriptor deliberately contains no compiled WASM bytes.  It mirrors the
/// same runtime-facing contract data that a compiled component would expose:
/// package artifact identity, ABI version, required host imports, and exported
/// application entry point.  Keeping this function pure makes fixture generation
/// reproducible and easy to audit in tests.
pub(super) fn artifact_fixture(
    fixture_id: &str,
    ability_id: &str,
    permission: &str,
) -> WasmComponentArtifactDescriptor {
    ApplicationImport::v0()
        .into_iter()
        .fold(
            WasmComponentArtifactDescriptor::new(
                format!("{fixture_id}.artifact"),
                WasmRuntimeArtifactRef::new(format!("pkg://{fixture_id}/component.wasm")),
                WasmArtifactDigest::sha256(format!("sha256-{fixture_id}")),
            )
            .abi(WasmAbiRequirement::new("0")),
            |descriptor, import| {
                descriptor.required_import(WasmImportRequirement::permissioned(
                    import,
                    permission.to_string(),
                ))
            },
        )
        .export(WasmExportDeclaration::ability(
            ApplicationExport::Start,
            ability_id.to_string(),
        ))
}

/// Build a host command result with stable harness metadata.
///
/// Runtime tests use the `reason_code` metadata as the audit key for expected
/// outcomes.  The status carries the behavioral result, while metadata records
/// which local harness path produced the result without leaking command payloads
/// or provider-specific details.
pub(super) fn local_result(
    status: ApplicationHostCommandStatus,
    output: Value,
    trace: Option<TraceContext>,
    reason_code: &str,
) -> ApplicationHostCommandResult {
    let mut result = ApplicationHostCommandResult::ok(output, trace);
    result.status = status;
    result
        .metadata
        .insert("reason_code".into(), reason_code.into());
    result
        .metadata
        .insert("harness".into(), "wasm_guest_runtime".into());
    result
}

/// Create a deterministic lookup key for a registered mock host import result.
///
/// The key is derived only from the ABI import label plus provider-neutral
/// service and operation identifiers.  Capability and payload values are not
/// part of the key so authorization and payload redaction stay independently
/// testable.
pub(super) fn outcome_key(import: &ApplicationImport, service_id: &str, operation: &str) -> String {
    format!(
        "{}|{}|{}",
        import.as_name(),
        sanitize_label(service_id),
        sanitize_label(operation)
    )
}

/// Map fixture categories to generic application ability kinds.
pub(super) fn ability_kind(kind: &WasmExampleFixtureKind) -> ApplicationAbilityKind {
    match kind {
        WasmExampleFixtureKind::GenUiRender => ApplicationAbilityKind::Ui,
        WasmExampleFixtureKind::Headless
        | WasmExampleFixtureKind::MemoryContextImport
        | WasmExampleFixtureKind::ServiceUnavailable => ApplicationAbilityKind::Headless,
    }
}

/// Return the generic permission label exercised by each fixture category.
pub(super) fn permission_for_kind(kind: &WasmExampleFixtureKind) -> &'static str {
    match kind {
        WasmExampleFixtureKind::Headless => "trace.emit",
        WasmExampleFixtureKind::GenUiRender => "ui.render",
        WasmExampleFixtureKind::MemoryContextImport => "context.read",
        WasmExampleFixtureKind::ServiceUnavailable => "service.call",
    }
}

/// Return the provider-neutral operation label exercised by a fixture category.
pub(super) fn operation_for_kind(kind: &WasmExampleFixtureKind) -> &'static str {
    match kind {
        WasmExampleFixtureKind::Headless => "start",
        WasmExampleFixtureKind::GenUiRender => "render",
        WasmExampleFixtureKind::MemoryContextImport => "context_snapshot",
        WasmExampleFixtureKind::ServiceUnavailable => "unavailable",
    }
}

/// Recursively sanitize JSON values before they enter logs or fixture reports.
///
/// Delegates to the canonical `macaca_proto::audit_redaction` harness policy so
/// WASM fixture exports and host-import replay share one redaction Strategy.
pub(super) fn sanitize_json(value: Value) -> Value {
    sanitize_json_for_harness(value)
}

/// Redact sensitive marker words from free-form text.
pub(super) fn sanitize_text(value: impl Into<String>) -> String {
    sanitize_string_for_audit(value.into())
}

/// Normalize labels into the conservative character set accepted by fixtures.
///
/// Slash is converted to colon because WIT-style and service labels already use
/// colon-delimited namespaces.  Unsupported characters are dropped instead of
/// escaped, which keeps generated IDs stable and avoids introducing
/// application-specific encoding rules.
pub(super) fn sanitize_label(value: impl AsRef<str>) -> String {
    value
        .as_ref()
        .trim()
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | ':') {
                Some(ch)
            } else if ch == '/' {
                Some(':')
            } else {
                None
            }
        })
        .collect()
}
