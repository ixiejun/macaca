//! Workspace-scope decoration for application-owned UI service calls.
//!
//! The application UI bridge knows the route `app_id`, and Web bootstrap
//! records the matching [`AppWorkspace`] in `state.config.app_workspaces`.
//! Model-facing Workbench tools must not choose arbitrary host roots, so this
//! module decorates generic workspace-scoped service payloads with the
//! platform-owned application workspace root immediately before host dispatch.
//! The logic is intentionally data-shape based and application-agnostic: it
//! never branches on an application name, workflow, provider, or model.

use std::path::Path;

use serde_json::{Map, Value};
use tracing::debug;

/// Inject the registered application workspace root into generic workbench
/// service DTO shapes.
///
/// The function is a host-side Decorator: it preserves the caller's command
/// intent while replacing any caller-provided workspace root with the trusted
/// root registered by Macaca Web. If the app workspace is unavailable, the
/// payload is returned unchanged so the service runtime can produce the
/// structured missing/invalid-argument result for that command.
pub(crate) fn decorate_workspace_scoped_payload(
    service_id: &str,
    operation: &str,
    payload: Value,
    workspace_root: Option<&Path>,
) -> Value {
    let Some(root) = workspace_root else {
        return payload;
    };
    let root = root.display().to_string();
    let mut payload = match payload {
        Value::Object(map) => map,
        other => {
            // Workspace-scoped services accept structured command DTOs.  If a
            // caller sends a scalar/array/null payload, the bridge should not
            // normalize it into an empty object because that would hide the
            // actual malformed payload shape from the owning service's
            // validation path.
            return other;
        }
    };
    match service_id {
        "service.file" => decorate_file_payload(&mut payload, &root),
        "service.git" | "service.code_intelligence" => {
            payload.insert("workspace_root".into(), Value::String(root));
        }
        "service.process" => decorate_process_payload(&mut payload, &root),
        "service.sandbox" => {
            payload.insert("workspace_root".into(), Value::String(root));
        }
        _ => {}
    }
    debug!(
        service_id,
        operation,
        workspace_scoped = matches!(
            service_id,
            "service.file"
                | "service.git"
                | "service.code_intelligence"
                | "service.process"
                | "service.sandbox"
        ),
        "application UI bridge decorated workspace-scoped payload"
    );
    Value::Object(payload)
}

fn decorate_file_payload(payload: &mut Map<String, Value>, workspace_root: &str) {
    decorate_nested_object(payload, "target", workspace_root);
    decorate_nested_object(payload, "source", workspace_root);
}

fn decorate_process_payload(payload: &mut Map<String, Value>, workspace_root: &str) {
    decorate_nested_object(payload, "sandbox", workspace_root);
}

fn decorate_nested_object(payload: &mut Map<String, Value>, key: &str, workspace_root: &str) {
    if let Some(Value::Object(target)) = payload.get_mut(key) {
        target.insert(
            "workspace_root".into(),
            Value::String(workspace_root.to_string()),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::json;

    use super::decorate_workspace_scoped_payload;

    #[test]
    fn app_ui_bridge_workspace_decorator_injects_file_target_root() {
        let payload = decorate_workspace_scoped_payload(
            "service.file",
            "file.write",
            json!({"target":{"path":"src/main.js","follow_symlinks":false},"content":"hello"}),
            Some(Path::new("/macaca/workspaces/app-id")),
        );

        assert_eq!(
            payload["target"]["workspace_root"],
            "/macaca/workspaces/app-id"
        );
        assert_eq!(payload["target"]["path"], "src/main.js");
    }

    #[test]
    fn app_ui_bridge_workspace_decorator_overrides_untrusted_git_root() {
        let payload = decorate_workspace_scoped_payload(
            "service.git",
            "git.status",
            json!({"workspace_root":"/Users/quantum/Code/dev"}),
            Some(Path::new("/macaca/workspaces/app-id")),
        );

        assert_eq!(payload["workspace_root"], "/macaca/workspaces/app-id");
    }

    #[test]
    fn app_ui_bridge_workspace_decorator_injects_process_sandbox_root() {
        let payload = decorate_workspace_scoped_payload(
            "service.process",
            "process.exec",
            json!({"executable":"npm","args":["test"],"sandbox":{"cwd":"frontend"}}),
            Some(Path::new("/macaca/workspaces/app-id")),
        );

        assert_eq!(
            payload["sandbox"]["workspace_root"],
            "/macaca/workspaces/app-id"
        );
        assert_eq!(payload["sandbox"]["cwd"], "frontend");
    }

    #[test]
    fn app_ui_bridge_workspace_decorator_leaves_non_workspace_services_unchanged() {
        let payload = decorate_workspace_scoped_payload(
            "service.llm",
            "llm.chat",
            json!({"model":"provider:model"}),
            Some(Path::new("/macaca/workspaces/app-id")),
        );

        assert_eq!(payload, json!({"model":"provider:model"}));
    }

    #[test]
    fn app_ui_bridge_workspace_decorator_preserves_non_object_payloads() {
        let payload = decorate_workspace_scoped_payload(
            "service.file",
            "file.write",
            json!("not-an-object"),
            Some(Path::new("/macaca/workspaces/app-id")),
        );

        assert_eq!(payload, json!("not-an-object"));
    }
}
