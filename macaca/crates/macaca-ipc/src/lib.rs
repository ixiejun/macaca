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
pub mod local_service;
pub mod nats;
pub mod service_bus;
pub mod transport;

pub use bus::{MessageReceiver, MessageSender};
pub use local::{LocalBus, LocalReceiver, LocalSender};
pub use local_service::{LocalServiceHandler, LocalServiceTransport};
pub use nats::{NatsBus, NatsReceiver, NatsSender};
pub use service_bus::{
    DeadlineBusMiddleware, InMemoryServiceBusTraceSink, ServiceBus, ServiceBusContext,
    ServiceBusMiddleware, ServiceBusTraceEvent, ServiceBusTraceSink, StaticDenyBusMiddleware,
    TraceRequiredBusMiddleware,
};
pub use transport::{
    DynMessageReceiver, DynMessageSender, DynServiceTransport, IpcTransport, IpcTransportConfig,
    IpcTransportFactory, IpcTransportKind, ServiceTransport,
};
