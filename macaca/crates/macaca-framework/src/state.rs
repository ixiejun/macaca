//! StateModule — self-describing serialization for all stateful components.
//!
//! Any component that needs to persist its state across sessions implements
//! `StateModule`. This enables recursive serialization: when an Agent's
//! state is saved, its Memory, Toolkit, and PlanNotebook are automatically
//! included.
//!
//! Inspired by PyTorch's `nn.Module.state_dict()` pattern and AgentScope's
//! `StateModule` class.

use serde_json::Value;

/// Trait for components with serializable state.
///
/// Implementors declare which parts of their state should be persisted
/// and how to restore them. The framework automatically discovers nested
/// `StateModule` implementors and includes them recursively.
///
/// # Example
///
/// ```rust,ignore
/// struct MyMemory {
///     messages: Vec<Msg>,
///     summary: Option<String>,
/// }
///
/// impl StateModule for MyMemory {
///     fn state_dict(&self) -> Value {
///         serde_json::json!({
///             "messages": self.messages,
///             "summary": self.summary,
///         })
///     }
///
///     fn load_state_dict(&mut self, state: Value) -> Result<(), StateError> {
///         if let Some(msgs) = state.get("messages") {
///             self.messages = serde_json::from_value(msgs.clone())
///                 .map_err(|e| StateError::DeserializeFailed(e.to_string()))?;
///         }
///         self.summary = state.get("summary")
///             .and_then(|v| v.as_str())
///             .map(|s| s.to_string());
///         Ok(())
///     }
/// }
/// ```
pub trait StateModule: Send + Sync {
    /// Serialize the component's state to a JSON value.
    ///
    /// The returned value should contain all data needed to restore
    /// the component to its current state via `load_state_dict`.
    fn state_dict(&self) -> Value;

    /// Restore the component's state from a previously saved JSON value.
    ///
    /// # Errors
    ///
    /// Returns `StateError` if the state cannot be deserialized or is
    /// incompatible with the current version.
    fn load_state_dict(&mut self, state: Value) -> Result<(), StateError>;

    /// Name of this module (for nested state key naming).
    ///
    /// Defaults to the type name. Override for custom keys in the
    /// parent's state_dict.
    fn module_name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// Errors that can occur during state serialization/deserialization.
#[derive(Debug, Clone, thiserror::Error)]
pub enum StateError {
    /// Failed to serialize state.
    #[error("Serialize failed: {0}")]
    SerializeFailed(String),

    /// Failed to deserialize state.
    #[error("Deserialize failed: {0}")]
    DeserializeFailed(String),

    /// State version mismatch.
    #[error("Version mismatch: expected {expected}, got {got}")]
    VersionMismatch { expected: String, got: String },

    /// Missing required field in state.
    #[error("Missing field: {0}")]
    MissingField(String),
}

/// Helper to build a composite state_dict from multiple named sub-modules.
///
/// # Example
///
/// ```rust,ignore
/// fn state_dict(&self) -> Value {
///     composite_state_dict(&[
///         ("memory", &self.memory),
///         ("toolkit", &self.toolkit),
///     ])
/// }
/// ```
pub fn composite_state_dict(modules: &[(&str, &dyn StateModule)]) -> Value {
    let mut map = serde_json::Map::new();
    for (name, module) in modules {
        map.insert(name.to_string(), module.state_dict());
    }
    Value::Object(map)
}

/// Helper to load a composite state_dict into multiple named sub-modules.
pub fn load_composite_state_dict(
    state: &Value,
    modules: &mut [(&str, &mut dyn StateModule)],
) -> Result<(), StateError> {
    for (name, module) in modules.iter_mut() {
        if let Some(sub_state) = state.get(*name) {
            module.load_state_dict(sub_state.clone())?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    struct Counter {
        count: u64,
        label: String,
    }

    impl StateModule for Counter {
        fn state_dict(&self) -> Value {
            serde_json::json!({
                "count": self.count,
                "label": self.label,
            })
        }

        fn load_state_dict(&mut self, state: Value) -> Result<(), StateError> {
            self.count = state
                .get("count")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| StateError::MissingField("count".into()))?;
            self.label = state
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(())
        }
    }

    #[test]
    fn test_state_roundtrip() {
        let counter = Counter {
            count: 42,
            label: "test".into(),
        };
        let state = counter.state_dict();

        let mut restored = Counter {
            count: 0,
            label: String::new(),
        };
        restored.load_state_dict(state).unwrap();

        assert_eq!(restored.count, 42);
        assert_eq!(restored.label, "test");
    }

    #[test]
    fn test_composite_state() {
        let a = Counter {
            count: 1,
            label: "a".into(),
        };
        let b = Counter {
            count: 2,
            label: "b".into(),
        };

        let state = composite_state_dict(&[("alpha", &a), ("beta", &b)]);
        assert_eq!(state["alpha"]["count"], 1);
        assert_eq!(state["beta"]["label"], "b");

        let mut a2 = Counter {
            count: 0,
            label: String::new(),
        };
        let mut b2 = Counter {
            count: 0,
            label: String::new(),
        };
        load_composite_state_dict(&state, &mut [("alpha", &mut a2), ("beta", &mut b2)]).unwrap();

        assert_eq!(a2.count, 1);
        assert_eq!(b2.label, "b");
    }

    #[test]
    fn test_missing_field_error() {
        let mut counter = Counter {
            count: 0,
            label: String::new(),
        };
        let result = counter.load_state_dict(serde_json::json!({"label": "x"}));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StateError::MissingField(_)));
    }

    #[test]
    fn test_composite_state_dict_multiple_modules() {
        let a = Counter {
            count: 10,
            label: "alpha".into(),
        };
        let b = Counter {
            count: 20,
            label: "beta".into(),
        };
        let c = Counter {
            count: 30,
            label: "gamma".into(),
        };

        let state = composite_state_dict(&[("a", &a), ("b", &b), ("c", &c)]);

        // All three modules present.
        assert_eq!(state["a"]["count"], 10);
        assert_eq!(state["b"]["count"], 20);
        assert_eq!(state["c"]["count"], 30);
        assert_eq!(state["a"]["label"], "alpha");
        assert_eq!(state["c"]["label"], "gamma");

        // Round-trip load.
        let mut a2 = Counter {
            count: 0,
            label: String::new(),
        };
        let mut b2 = Counter {
            count: 0,
            label: String::new(),
        };
        let mut c2 = Counter {
            count: 0,
            label: String::new(),
        };
        load_composite_state_dict(
            &state,
            &mut [("a", &mut a2), ("b", &mut b2), ("c", &mut c2)],
        )
        .unwrap();
        assert_eq!(a2.count, 10);
        assert_eq!(b2.count, 20);
        assert_eq!(c2.count, 30);
    }

    #[test]
    fn test_load_composite_missing_module() {
        // State only has "a" key; "b" module is missing from the data.
        let state = serde_json::json!({ "a": { "count": 5, "label": "a" } });

        let mut a = Counter {
            count: 0,
            label: String::new(),
        };
        let mut b = Counter {
            count: 99,
            label: "original".into(),
        };

        // load_composite_state_dict silently skips missing keys.
        load_composite_state_dict(&state, &mut [("a", &mut a), ("b", &mut b)]).unwrap();

        assert_eq!(a.count, 5);
        // b remains unchanged because "b" key was missing.
        assert_eq!(b.count, 99);
        assert_eq!(b.label, "original");
    }

    #[test]
    fn test_state_module_empty_roundtrip() {
        // Counter with zero/empty values.
        let counter = Counter {
            count: 0,
            label: String::new(),
        };
        let state = counter.state_dict();

        assert_eq!(state["count"], 0);
        assert_eq!(state["label"], "");

        let mut restored = Counter {
            count: 999,
            label: "dirty".into(),
        };
        restored.load_state_dict(state).unwrap();
        assert_eq!(restored.count, 0);
        assert_eq!(restored.label, "");
    }
}
