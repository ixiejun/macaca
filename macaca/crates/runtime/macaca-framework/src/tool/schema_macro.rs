//! `tool_schema!` declarative macro for JSON Schema construction.
//!
//! The macro is `#[macro_export]` so downstream crates invoke it as
//! `macaca_framework::tool_schema!` regardless of this module's location inside the
//! Facade tree. It builds standard OpenAI-style function parameter objects with an
//! auto-derived `required` array based on presence of `"default"` keys.

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
