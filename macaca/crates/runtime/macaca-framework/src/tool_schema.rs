// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Exported helper macro for tool parameter schemas.
//!
//! The macro stays in its own module so the toolkit registry can remain focused
//! on runtime behavior while call sites keep using `macaca_framework::tool_schema!`.

/// Macro to conveniently construct a JSON Schema object for tool parameters.
///
/// # Example
/// ```rust
/// use macaca_framework::tool_schema;
/// let schema = tool_schema!({
///     "query" => { "type": "string", "description": "Search query" },
///     "limit" => { "type": "integer", "description": "Max results", "default": 5 }
/// });
/// ```
#[macro_export]
macro_rules! tool_schema {
    ({}) => {{
        serde_json::json!({
            "type": "object",
            "properties": serde_json::Map::<String, serde_json::Value>::new(),
            "required": Vec::<String>::new()
        })
    }};
    ({ $( $name:tt => $props:tt ),* $(,)? }) => {{
        let mut properties = serde_json::Map::new();
        let mut required = Vec::<String>::new();
        $(
            let prop_val: serde_json::Value = serde_json::json!($props);
            // If no "default" key exists, the field is required.
            if !prop_val.as_object().map_or(false, |o| o.contains_key("default")) {
                required.push($name.to_string());
            }
            properties.insert($name.to_string(), prop_val);
        )*
        serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required
        })
    }};
}
