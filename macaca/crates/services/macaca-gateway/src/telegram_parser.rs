//! Telegram message parsing helpers.

use macaca_proto::types::GatewayEvent;

/// Parse a raw Telegram message text into a [`GatewayEvent`].
///
/// - `/status [task_id]` maps to [`GatewayEvent::StatusQuery`]
/// - Any other `/command [args...]` maps to [`GatewayEvent::Command`]
/// - Plain text maps to [`GatewayEvent::TaskRequest`]
pub(crate) fn parse_message(text: &str, user_id: &str, channel_id: &str) -> GatewayEvent {
    let text = text.trim();
    if let Some(rest) = text.strip_prefix('/') {
        let (cmd_raw, arg_str) = match rest.split_once(' ') {
            Some((c, a)) => (c, a),
            None => (rest, ""),
        };
        let command = cmd_raw
            .split_once('@')
            .map(|(c, _)| c)
            .unwrap_or(cmd_raw)
            .to_string();
        let args: Vec<String> = arg_str.split_whitespace().map(|s| s.to_string()).collect();

        if command == "status" {
            return GatewayEvent::StatusQuery {
                user_id: user_id.into(),
                channel_id: channel_id.into(),
                task_id: args.first().and_then(|id| {
                    uuid::Uuid::parse_str(id)
                        .ok()
                        .map(macaca_proto::types::TaskId)
                }),
            };
        }

        return GatewayEvent::Command {
            user_id: user_id.into(),
            channel_id: channel_id.into(),
            command,
            args,
        };
    }

    GatewayEvent::TaskRequest {
        user_id: user_id.into(),
        channel_id: channel_id.into(),
        content: text.to_string(),
    }
}
