//! Pure conversion policies used by the built-in time providers.
//!
//! These functions deliberately avoid host locale and time-zone databases. A
//! provider can replace this module with an adapter while preserving the
//! service contract and its deterministic error behavior.

use macaca_proto::{ServiceError, ServiceResult};

/// Resolve only explicit UTC or fixed-offset zones without platform data.
pub(crate) fn resolve_timezone(payload: &serde_json::Value) -> ServiceResult<serde_json::Value> {
    let zone = payload
        .get("zone_query")
        .and_then(|value| value.as_str())
        .ok_or_else(|| ServiceError::InvalidArgument("timezone query is required".into()))?;
    let offset = timezone_offset_seconds(zone).ok_or_else(|| {
        ServiceError::InvalidArgument("timezone is unavailable in this provider".into())
    })?;
    Ok(
        serde_json::json!({"status":"success","zone_id":zone,"offset_seconds":offset,"data_version":"fixed-offset-v1"}),
    )
}

/// Convert an instant through an explicit fixed offset rather than host state.
pub(crate) fn convert_timezone(payload: &serde_json::Value) -> ServiceResult<serde_json::Value> {
    let epoch_millis = payload
        .pointer("/instant/epoch_millis")
        .and_then(|value| value.as_i64())
        .ok_or_else(|| ServiceError::InvalidArgument("instant is required".into()))?;
    let zone = payload
        .pointer("/target_timezone/zone_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| ServiceError::InvalidArgument("target timezone is required".into()))?;
    let offset = timezone_offset_seconds(zone).ok_or_else(|| {
        ServiceError::InvalidArgument("timezone is unavailable in this provider".into())
    })?;
    Ok(
        serde_json::json!({"status":"success","epoch_millis":epoch_millis,"zone_id":zone,"offset_seconds":offset,"timezone_data_version":"fixed-offset-v1"}),
    )
}

/// Report unsupported calendars explicitly until an adapter is installed.
pub(crate) fn calendar_convert(payload: &serde_json::Value) -> ServiceResult<serde_json::Value> {
    let calendar = payload
        .pointer("/target_calendar/calendar_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| ServiceError::InvalidArgument("target calendar is required".into()))?;
    if !matches!(calendar, "iso8601" | "gregorian") {
        return Ok(
            serde_json::json!({"status":"unsupported","reason":"calendar_adapter_not_installed"}),
        );
    }
    Ok(serde_json::json!({"status":"success","calendar_id":calendar}))
}

/// Format named stable references so raw user patterns never enter diagnostics.
pub(crate) fn format_time(payload: &serde_json::Value) -> ServiceResult<serde_json::Value> {
    let epoch_millis = payload
        .pointer("/instant/epoch_millis")
        .and_then(|value| value.as_i64())
        .ok_or_else(|| ServiceError::InvalidArgument("instant is required".into()))?;
    let format_ref = payload
        .pointer("/format/pattern_ref")
        .and_then(|value| value.as_str())
        .unwrap_or("format:rfc3339");
    if format_ref != "format:rfc3339" {
        return Ok(
            serde_json::json!({"status":"unsupported","reason":"format_reference_unsupported"}),
        );
    }
    let instant = chrono::DateTime::from_timestamp_millis(epoch_millis)
        .ok_or_else(|| ServiceError::InvalidArgument("instant is out of range".into()))?;
    Ok(
        serde_json::json!({"status":"success","formatted":instant.to_rfc3339(),"format_ref":format_ref,"locale":"invariant"}),
    )
}

/// Parse RFC3339 and return only a normalized instant, never the input text.
pub(crate) fn parse_time(payload: &serde_json::Value) -> ServiceResult<serde_json::Value> {
    let input = payload
        .get("input_ref")
        .and_then(|value| value.as_str())
        .ok_or_else(|| ServiceError::InvalidArgument("parse input is required".into()))?;
    let parsed = chrono::DateTime::parse_from_rfc3339(input)
        .map_err(|_| ServiceError::InvalidArgument("strict RFC3339 parsing failed".into()))?;
    Ok(
        serde_json::json!({"status":"success","epoch_millis":parsed.timestamp_millis(),"timezone_data_version":"fixed-offset-v1"}),
    )
}

/// Interpret the restricted fixed-offset grammar used by the default adapter.
fn timezone_offset_seconds(zone: &str) -> Option<i32> {
    if matches!(zone, "UTC" | "Etc/UTC" | "Z") {
        return Some(0);
    }
    let raw = zone.strip_prefix("UTC")?;
    let sign = match raw.as_bytes().first()? {
        b'+' => 1_i32,
        b'-' => -1_i32,
        _ => return None,
    };
    let (hours, minutes) = raw[1..].split_once(':')?;
    let hours = hours.parse::<i32>().ok()?;
    let minutes = minutes.parse::<i32>().ok()?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    Some(sign * (hours * 3_600 + minutes * 60))
}
