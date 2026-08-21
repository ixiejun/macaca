//! Provider-neutral State Specification for transcription streaming sessions.
//!
//! It validates transitions and chunk ordering before an adapter can contact a
//! speech provider or retain media. Session IDs, chunk bytes, and transcripts
//! remain outside this pure state machine.

use serde::{Deserialize, Serialize};

/// Bounded stream lifecycle states shared by mock, remote, and local adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionStreamState {
    Planned,
    Started,
    AcceptingChunks,
    Draining,
    Finished,
    Cancelled,
    Failed,
    TimedOut,
    Unavailable,
}

/// Bounded asynchronous job lifecycle for batch, redaction, export, and handoff work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionJobState {
    Planned,
    Started,
    Draining,
    Finished,
    Cancelled,
    Failed,
    TimedOut,
    Unavailable,
    Replay,
}

/// Generic job actions shared by all asynchronous transcription operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionJobAction {
    Start,
    Drain,
    Finish,
    Cancel,
    Timeout,
    Fail,
    ResumeReplay,
}

/// Commands that change session state without carrying raw media.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionStreamAction {
    Start,
    Append,
    Finish,
    Cancel,
    Timeout,
    Fail,
}

/// Stable rejection reasons returned before provider dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionStreamTransitionFailure {
    InvalidState,
    ChunkOrderConflict,
}

/// Apply one lifecycle action and advance the opaque chunk sequence when valid.
pub fn transition_transcription_stream(
    state: TranscriptionStreamState,
    next_sequence: u64,
    action: TranscriptionStreamAction,
    submitted_sequence: Option<u64>,
) -> Result<(TranscriptionStreamState, u64), TranscriptionStreamTransitionFailure> {
    match action {
        TranscriptionStreamAction::Start if state == TranscriptionStreamState::Planned => {
            Ok((TranscriptionStreamState::AcceptingChunks, next_sequence))
        }
        TranscriptionStreamAction::Append if state == TranscriptionStreamState::AcceptingChunks => {
            (submitted_sequence == Some(next_sequence))
                .then_some((state, next_sequence.saturating_add(1)))
                .ok_or(TranscriptionStreamTransitionFailure::ChunkOrderConflict)
        }
        TranscriptionStreamAction::Finish if state == TranscriptionStreamState::AcceptingChunks => {
            Ok((TranscriptionStreamState::Finished, next_sequence))
        }
        TranscriptionStreamAction::Cancel
            if matches!(
                state,
                TranscriptionStreamState::Planned
                    | TranscriptionStreamState::Started
                    | TranscriptionStreamState::AcceptingChunks
                    | TranscriptionStreamState::Draining
            ) =>
        {
            Ok((TranscriptionStreamState::Cancelled, next_sequence))
        }
        TranscriptionStreamAction::Timeout
            if matches!(
                state,
                TranscriptionStreamState::Started
                    | TranscriptionStreamState::AcceptingChunks
                    | TranscriptionStreamState::Draining
            ) =>
        {
            Ok((TranscriptionStreamState::TimedOut, next_sequence))
        }
        TranscriptionStreamAction::Fail
            if !matches!(
                state,
                TranscriptionStreamState::Finished | TranscriptionStreamState::Cancelled
            ) =>
        {
            Ok((TranscriptionStreamState::Failed, next_sequence))
        }
        _ => Err(TranscriptionStreamTransitionFailure::InvalidState),
    }
}

/// Advance an asynchronous job without exposing source, transcript, or artifact content.
pub fn transition_transcription_job(
    state: TranscriptionJobState,
    action: TranscriptionJobAction,
) -> Result<TranscriptionJobState, TranscriptionStreamTransitionFailure> {
    match (state, action) {
        (TranscriptionJobState::Planned, TranscriptionJobAction::Start) => {
            Ok(TranscriptionJobState::Started)
        }
        (TranscriptionJobState::Started, TranscriptionJobAction::Drain) => {
            Ok(TranscriptionJobState::Draining)
        }
        (TranscriptionJobState::Draining, TranscriptionJobAction::Finish) => {
            Ok(TranscriptionJobState::Finished)
        }
        (
            TranscriptionJobState::Planned
            | TranscriptionJobState::Started
            | TranscriptionJobState::Draining,
            TranscriptionJobAction::Cancel,
        ) => Ok(TranscriptionJobState::Cancelled),
        (
            TranscriptionJobState::Started | TranscriptionJobState::Draining,
            TranscriptionJobAction::Timeout,
        ) => Ok(TranscriptionJobState::TimedOut),
        (
            TranscriptionJobState::Planned
            | TranscriptionJobState::Started
            | TranscriptionJobState::Draining
            | TranscriptionJobState::Replay,
            TranscriptionJobAction::Fail,
        ) => Ok(TranscriptionJobState::Failed),
        (
            TranscriptionJobState::Started
            | TranscriptionJobState::Draining
            | TranscriptionJobState::TimedOut,
            TranscriptionJobAction::ResumeReplay,
        ) => Ok(TranscriptionJobState::Replay),
        _ => Err(TranscriptionStreamTransitionFailure::InvalidState),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_state_requires_ordered_chunks_and_prevents_terminal_mutation() {
        let (state, cursor) = transition_transcription_stream(
            TranscriptionStreamState::Planned,
            0,
            TranscriptionStreamAction::Start,
            None,
        )
        .unwrap();
        let (state, cursor) = transition_transcription_stream(
            state,
            cursor,
            TranscriptionStreamAction::Append,
            Some(0),
        )
        .unwrap();
        assert_eq!(cursor, 1);
        assert_eq!(
            transition_transcription_stream(
                state,
                cursor,
                TranscriptionStreamAction::Append,
                Some(3)
            ),
            Err(TranscriptionStreamTransitionFailure::ChunkOrderConflict)
        );
        let (finished, _) =
            transition_transcription_stream(state, cursor, TranscriptionStreamAction::Finish, None)
                .unwrap();
        assert_eq!(
            transition_transcription_stream(
                finished,
                cursor,
                TranscriptionStreamAction::Append,
                Some(cursor)
            ),
            Err(TranscriptionStreamTransitionFailure::InvalidState)
        );
    }

    #[test]
    fn job_state_models_cancellation_timeout_and_replay_without_payloads() {
        let started = transition_transcription_job(
            TranscriptionJobState::Planned,
            TranscriptionJobAction::Start,
        )
        .unwrap();
        let timed_out =
            transition_transcription_job(started, TranscriptionJobAction::Timeout).unwrap();
        let replay =
            transition_transcription_job(timed_out, TranscriptionJobAction::ResumeReplay).unwrap();
        assert_eq!(replay, TranscriptionJobState::Replay);
        assert_eq!(
            transition_transcription_job(
                TranscriptionJobState::Finished,
                TranscriptionJobAction::Cancel
            ),
            Err(TranscriptionStreamTransitionFailure::InvalidState)
        );
    }
}
