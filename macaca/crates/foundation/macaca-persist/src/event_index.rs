use macaca_proto::types::EventEntry;
use macaca_proto::EventLogQuery;

const SOURCE_PREFIX: &str = "events_by_source/";
const AGENT_PREFIX: &str = "events_by_agent/";
const TYPE_PREFIX: &str = "events_by_type/";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedIndex {
    Agent,
    Source,
    EventType,
}

pub fn select_index(query: &EventLogQuery) -> Option<SelectedIndex> {
    if query.agent.is_some() {
        Some(SelectedIndex::Agent)
    } else if query.source.is_some() {
        Some(SelectedIndex::Source)
    } else if query.event_type.is_some() {
        Some(SelectedIndex::EventType)
    } else {
        None
    }
}

pub fn index_key(index: SelectedIndex, session_id: &str, value: &str, seq: u64) -> String {
    format!(
        "{}{}/{}/{:08}",
        index_prefix(index),
        session_id,
        encode_segment(value),
        seq
    )
}

pub fn index_prefix_for_query(query: &EventLogQuery, index: SelectedIndex) -> Option<String> {
    let value = match index {
        SelectedIndex::Agent => query.agent.as_deref(),
        SelectedIndex::Source => query.source.as_deref(),
        SelectedIndex::EventType => query.event_type.as_deref(),
    }?;
    Some(format!(
        "{}{}/{}/",
        index_prefix(index),
        query.session_id,
        encode_segment(value)
    ))
}

pub fn seq_from_key(key: &str) -> u64 {
    key.rsplit('/')
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0)
}

pub fn agent_from_payload(payload: &serde_json::Value) -> Option<String> {
    payload
        .get("agent")
        .or_else(|| payload.get("agent_tab"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn event_matches(
    entry: &EventEntry,
    query: &EventLogQuery,
    selected: Option<SelectedIndex>,
) -> bool {
    if selected != Some(SelectedIndex::Source)
        && query
            .source
            .as_deref()
            .is_some_and(|source| entry.source != source)
    {
        return false;
    }
    if selected != Some(SelectedIndex::EventType)
        && query
            .event_type
            .as_deref()
            .is_some_and(|event_type| entry.event_type != event_type)
    {
        return false;
    }
    if selected != Some(SelectedIndex::Agent) {
        if let Some(agent) = query.agent.as_deref() {
            return agent_from_payload(&entry.payload)
                .as_deref()
                .is_some_and(|value| value == agent);
        }
    }
    true
}

fn index_prefix(index: SelectedIndex) -> &'static str {
    match index {
        SelectedIndex::Agent => AGENT_PREFIX,
        SelectedIndex::Source => SOURCE_PREFIX,
        SelectedIndex::EventType => TYPE_PREFIX,
    }
}

fn encode_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}
