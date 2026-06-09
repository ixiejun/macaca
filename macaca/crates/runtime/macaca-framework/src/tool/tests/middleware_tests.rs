//! Middleware chain contract tests for [`Toolkit::call_tool`].

use std::sync::{Arc, Mutex};

use crate::message::ContentBlock;
use crate::tool::{ToolError, Toolkit};

use super::fixtures::{
    AddTool, EchoTool, FailBeforeMiddleware, LabeledMiddleware, ModifyAfterMiddleware,
    RecordingMiddleware, TrackingTool,
};

#[tokio::test]
async fn test_middleware_chain() {
    let before_calls = Arc::new(Mutex::new(Vec::<String>::new()));
    let after_calls = Arc::new(Mutex::new(Vec::<String>::new()));

    let mw = RecordingMiddleware {
        before_calls: before_calls.clone(),
        after_calls: after_calls.clone(),
    };

    let mut kit = Toolkit::new();
    kit.add_middleware(Box::new(mw));
    kit.register(Box::new(AddTool), None);

    kit.call_tool("add", serde_json::json!({"a": 1.0, "b": 2.0}))
        .await
        .unwrap();

    assert_eq!(*before_calls.lock().unwrap(), vec!["add"]);
    assert_eq!(*after_calls.lock().unwrap(), vec!["add"]);
}

#[tokio::test]
async fn test_middleware_before_error_short_circuits() {
    let called = Arc::new(Mutex::new(false));
    let mut kit = Toolkit::new();
    kit.add_middleware(Box::new(FailBeforeMiddleware));
    kit.register(
        Box::new(TrackingTool {
            called: called.clone(),
        }),
        None,
    );

    let err = kit
        .call_tool("tracking", serde_json::json!({}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::ExecutionFailed(_)));
    assert!(!*called.lock().unwrap());
}

#[tokio::test]
async fn test_middleware_after_modifies_response() {
    let mut kit = Toolkit::new();
    kit.register(Box::new(AddTool), None);
    kit.add_middleware(Box::new(ModifyAfterMiddleware));

    let resp = kit
        .call_tool("add", serde_json::json!({"a": 1.0, "b": 2.0}))
        .await
        .unwrap();

    if let ContentBlock::Text(tb) = &resp.content[0] {
        assert_eq!(tb.text, "modified_by_after");
    } else {
        panic!("expected TextBlock");
    }
}

#[tokio::test]
async fn test_multiple_middlewares_chain() {
    let before_log = Arc::new(Mutex::new(Vec::<String>::new()));
    let after_log = Arc::new(Mutex::new(Vec::<String>::new()));

    let mut kit = Toolkit::new();
    for label in ["mw1", "mw2", "mw3"] {
        kit.add_middleware(Box::new(LabeledMiddleware {
            label: label.to_string(),
            before_log: before_log.clone(),
            after_log: after_log.clone(),
        }));
    }
    kit.register(Box::new(EchoTool), None);

    kit.call_tool("echo", serde_json::json!({})).await.unwrap();

    assert_eq!(*before_log.lock().unwrap(), vec!["mw1", "mw2", "mw3"]);
    assert_eq!(*after_log.lock().unwrap(), vec!["mw1", "mw2", "mw3"]);
}
