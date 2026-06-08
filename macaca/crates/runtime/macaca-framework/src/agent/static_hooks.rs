//! Process-wide static hook registry.
//!
//! **Design pattern:** Singleton — one `OnceLock` registry applies class-level
//! hooks to every [`super::hooked::HookedAgent`] (AgentScope metaclass equivalent).

use std::sync::OnceLock;

use tokio::sync::RwLock as TokioRwLock;
use tracing::{debug, info};

use crate::message::Msg;

use super::error::AgentResult;
use super::hooks::Hook;

/// Process-wide global hook storage.
static GLOBAL_HOOK_REGISTRY: OnceLock<TokioRwLock<Vec<Box<dyn Hook>>>> = OnceLock::new();

fn global_registry() -> &'static TokioRwLock<Vec<Box<dyn Hook>>> {
    GLOBAL_HOOK_REGISTRY.get_or_init(|| TokioRwLock::new(Vec::new()))
}

/// Register a hook applied to all [`HookedAgent`](super::hooked::HookedAgent) instances.
pub async fn register_static_hook(hook: Box<dyn Hook>) {
    let mut guard = global_registry().write().await;
    guard.push(hook);
    debug!(
        target: "macaca_framework::agent",
        static_hook_count = guard.len(),
        "static hook registered"
    );
}

/// Clear all process-wide static hooks (test isolation).
pub async fn clear_static_hooks() {
    let mut guard = global_registry().write().await;
    let cleared = guard.len();
    guard.clear();
    if cleared > 0 {
        info!(
            target: "macaca_framework::agent",
            cleared,
            "static hooks cleared"
        );
    }
}

/// Run static global pre-reply hooks.
pub(crate) async fn run_static_pre_reply(mut msg: Msg) -> AgentResult<Msg> {
    let hooks = global_registry().read().await;
    for hook in hooks.iter() {
        msg = hook.pre_reply(msg).await?;
    }
    Ok(msg)
}

/// Run static global post-reply hooks.
pub(crate) async fn run_static_post_reply(mut msg: Msg) -> AgentResult<Msg> {
    let hooks = global_registry().read().await;
    for hook in hooks.iter() {
        msg = hook.post_reply(msg).await?;
    }
    Ok(msg)
}

/// Run static global pre-observe hooks.
pub(crate) async fn run_static_pre_observe(mut msg: Msg) -> AgentResult<Msg> {
    let hooks = global_registry().read().await;
    for hook in hooks.iter() {
        msg = hook.pre_observe(msg).await?;
    }
    Ok(msg)
}

/// Run static global post-observe hooks.
pub(crate) async fn run_static_post_observe() -> AgentResult<()> {
    let hooks = global_registry().read().await;
    for hook in hooks.iter() {
        hook.post_observe().await?;
    }
    Ok(())
}
