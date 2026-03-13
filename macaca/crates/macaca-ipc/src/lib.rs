//! `aos-ipc` — inter-agent communication layer for Agent OS.
//!
//! Provides two transports:
//! - [`local`] — in-process pub/sub via `tokio::sync::broadcast`.
//! - [`nats`]  — cross-process pub/sub via [NATS](https://nats.io).
//!
//! Both transports implement the [`bus::MessageSender`] and
//! [`bus::MessageReceiver`] traits defined in [`bus`].

pub mod bus;
pub mod local;
pub mod nats;

pub use bus::{MessageReceiver, MessageSender};
pub use local::{LocalBus, LocalReceiver, LocalSender};
pub use nats::{NatsBus, NatsReceiver, NatsSender};
