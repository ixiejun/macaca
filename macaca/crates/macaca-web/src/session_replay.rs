//! Session replay cursor primitives.

/// Immutable replay request state for EventLog-backed session reconstruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionReplayState {
    session_id: String,
    since_seq: u64,
    limit: usize,
}

impl SessionReplayState {
    /// Create replay state for a session.
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            since_seq: 0,
            limit: 500,
        }
    }

    /// Set the exclusive sequence cursor.
    pub fn since_seq(mut self, since_seq: u64) -> Self {
        self.since_seq = since_seq;
        self
    }

    /// Set the maximum number of events to replay.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Session identifier being replayed.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Exclusive EventLog sequence cursor.
    pub fn cursor(&self) -> u64 {
        self.since_seq
    }

    /// Maximum event count.
    pub fn replay_limit(&self) -> usize {
        self.limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_replay_cursor_and_limit() {
        let replay = SessionReplayState::new("session-1").since_seq(12).limit(25);

        assert_eq!(replay.session_id(), "session-1");
        assert_eq!(replay.cursor(), 12);
        assert_eq!(replay.replay_limit(), 25);
    }
}
