//! Local session-loop runtime owner for in-process task controllers.
//!
//! `service.execution_control` owns the authoritative state machine and audit
//! timeline for PlanLoop/WorkerLoop lifecycle events. This module owns only the
//! in-process shutdown flags and wake handles required by the current local
//! `macaca-task` controllers. Keeping those handles in `macaca-runtime-host`
//! prevents presentation shells from becoming execution-loop owners while still
//! allowing Web/CLI hosts to materialize local controllers during bootstrap.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use macaca_proto::ApplicationId;
use macaca_task::{
    PlanLoop, PlanLoopConfig, PlanLoopWaker, TaskBoard, TaskSpace, TodoStore, WorkerLoop,
    WorkerLoopConfig, WorkerLoopWaker,
};
use tokio::sync::RwLock;

/// Runtime-host owned local PlanLoop controller.
///
/// The controller is intentionally small: it carries the concrete local
/// `macaca-task` PlanLoop plus the shutdown flag reserved in the runtime host.
/// Shells may spawn the loop and adapt emitted events, but they do not create
/// the task workspace or own the loop-control registry.
pub struct LocalPlanLoopController {
    /// Concrete local PlanLoop instance created inside the runtime host.
    pub plan_loop: PlanLoop,
    /// Shutdown flag owned by the runtime host reservation registry.
    pub shutdown: Arc<AtomicBool>,
}

/// Runtime-host owned local WorkerLoop controller.
///
/// The controller mirrors `LocalPlanLoopController` for pull-based worker
/// execution. It keeps `TaskBoard` and `WorkerLoop` construction in the
/// runtime-host layer while allowing shells to adapt WorkerEvents to SSE,
/// session persistence, and service-backed agent execution.
pub struct LocalWorkerLoopController {
    /// Per-agent task board created by the runtime host.
    pub board: Arc<TaskBoard>,
    /// Concrete local WorkerLoop instance created inside the runtime host.
    pub worker_loop: WorkerLoop,
    /// Shutdown flag stored later in the runtime-host control registry.
    pub shutdown: Arc<AtomicBool>,
    /// Wake handle stored later in the runtime-host control registry.
    pub waker: WorkerLoopWaker,
}

/// Runtime-host owned local handles for session-scoped task controllers.
///
/// The owner follows a small Registry/Facade pattern:
/// - callers reserve loop slots before spawning local controller tasks;
/// - callers publish the wakers produced by controller construction;
/// - wake and shutdown paths use stable methods instead of touching maps.
#[derive(Default)]
pub struct SessionLoopLocalRuntime {
    /// Per-application PlanLoop shutdown flags.
    plan_loop_shutdowns: RwLock<HashMap<ApplicationId, Arc<AtomicBool>>>,
    /// Per-application WorkerLoop shutdown flags.
    worker_loop_shutdowns: RwLock<HashMap<ApplicationId, Vec<Arc<AtomicBool>>>>,
    /// Per-application PlanLoop wake handles.
    plan_loop_wakers: RwLock<HashMap<ApplicationId, PlanLoopWaker>>,
    /// Per-application WorkerLoop wake handles.
    worker_loop_wakers: RwLock<HashMap<ApplicationId, Vec<WorkerLoopWaker>>>,
}

impl SessionLoopLocalRuntime {
    /// Create an empty local runtime owner.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reserve a PlanLoop slot and return the shutdown flag for the controller.
    ///
    /// Returning `None` makes startup idempotent for callers that race to start
    /// the same application loop. The existing loop remains authoritative until
    /// `shutdown_application_loops` removes its slot.
    pub async fn reserve_plan_loop(
        &self,
        application_id: &ApplicationId,
    ) -> Option<Arc<AtomicBool>> {
        let mut shutdowns = self.plan_loop_shutdowns.write().await;
        if shutdowns.contains_key(application_id) {
            tracing::debug!(
                application_id = %application_id.0,
                "PlanLoop reservation skipped because a local controller already exists"
            );
            return None;
        }

        let shutdown = Arc::new(AtomicBool::new(false));
        shutdowns.insert(*application_id, Arc::clone(&shutdown));
        tracing::info!(
            application_id = %application_id.0,
            "PlanLoop local controller slot reserved in runtime host"
        );
        Some(shutdown)
    }

    /// Reserve and build a local PlanLoop controller for an application.
    ///
    /// This method is the local-runtime Factory for the current in-process
    /// `macaca-task` controller. It keeps `TaskSpace` construction inside
    /// runtime-host ownership while preserving the event Adapter boundary used
    /// by shells. Future remote/plugin Task Service implementations can replace
    /// this factory without making presentation shells depend on task internals.
    pub async fn reserve_local_plan_loop_controller(
        &self,
        application_id: ApplicationId,
        session_id: Option<String>,
        store: Arc<TodoStore>,
    ) -> Option<LocalPlanLoopController> {
        let shutdown = self.reserve_plan_loop(&application_id).await?;
        let plan_space = Arc::new(TaskSpace::for_session(application_id, session_id, store));
        let plan_loop = PlanLoop::with_components(plan_space, PlanLoopConfig::default());
        tracing::info!(
            application_id = %application_id.0,
            "PlanLoop local controller constructed in runtime host"
        );
        Some(LocalPlanLoopController {
            plan_loop,
            shutdown,
        })
    }

    /// Store the PlanLoop waker produced by local controller construction.
    pub async fn set_plan_loop_waker(&self, application_id: ApplicationId, waker: PlanLoopWaker) {
        self.plan_loop_wakers
            .write()
            .await
            .insert(application_id, waker);
        tracing::debug!(
            application_id = %application_id.0,
            "PlanLoop local waker registered in runtime host"
        );
    }

    /// Return whether WorkerLoops already exist for an application.
    pub async fn has_worker_loops(&self, application_id: &ApplicationId) -> bool {
        self.worker_loop_shutdowns
            .read()
            .await
            .contains_key(application_id)
    }

    /// Store WorkerLoop shutdown flags and wakers after local controllers spawn.
    pub async fn set_worker_loop_controls(
        &self,
        application_id: ApplicationId,
        shutdowns: Vec<Arc<AtomicBool>>,
        wakers: Vec<WorkerLoopWaker>,
    ) {
        let worker_count = wakers.len();
        self.worker_loop_shutdowns
            .write()
            .await
            .insert(application_id, shutdowns);
        self.worker_loop_wakers
            .write()
            .await
            .insert(application_id, wakers);
        tracing::info!(
            application_id = %application_id.0,
            worker_count,
            "WorkerLoop local controller controls registered in runtime host"
        );
    }

    /// Build one local WorkerLoop controller for a worker agent.
    ///
    /// This Factory method centralizes `TaskBoard` and `WorkerLoop`
    /// construction in runtime-host ownership. The returned controller is not
    /// registered until the caller passes its shutdown and waker handles to
    /// `set_worker_loop_controls`, which keeps all workers for an application
    /// published atomically after startup succeeds.
    pub fn build_local_worker_loop_controller(
        &self,
        application_id: ApplicationId,
        agent_name: String,
        session_id: Option<String>,
        store: Arc<TodoStore>,
    ) -> LocalWorkerLoopController {
        let board = Arc::new(TaskBoard::for_agent(
            application_id.clone(),
            agent_name.clone(),
            session_id,
            store,
        ));
        let worker_loop =
            WorkerLoop::with_components(Arc::clone(&board), WorkerLoopConfig::default());
        let waker = worker_loop.waker();
        let shutdown = Arc::new(AtomicBool::new(false));
        tracing::info!(
            application_id = %application_id.0,
            agent = %agent_name,
            "WorkerLoop local controller constructed in runtime host"
        );
        LocalWorkerLoopController {
            board,
            worker_loop,
            shutdown,
            waker,
        }
    }

    /// Wake the local PlanLoop if it has published a waker.
    pub async fn wake_plan_loop(&self, application_id: &ApplicationId) -> bool {
        if let Some(waker) = self.plan_loop_wakers.read().await.get(application_id) {
            waker.wake();
            tracing::debug!(
                application_id = %application_id.0,
                "PlanLoop local controller woken by runtime host"
            );
            return true;
        }

        tracing::debug!(
            application_id = %application_id.0,
            "PlanLoop wake skipped because no local waker is registered"
        );
        false
    }

    /// Wake all local WorkerLoop controllers if they have published wakers.
    pub async fn wake_worker_loops(&self, application_id: &ApplicationId) -> usize {
        let mut awakened = 0usize;
        if let Some(wakers) = self.worker_loop_wakers.read().await.get(application_id) {
            for waker in wakers {
                waker.wake();
                awakened += 1;
            }
        }

        tracing::debug!(
            application_id = %application_id.0,
            awakened,
            "WorkerLoop local controllers woken by runtime host"
        );
        awakened
    }

    /// Signal all local loops for an application and remove their handles.
    ///
    /// This method deliberately removes wakers and shutdown flags together so a
    /// subsequent session can reserve fresh local controllers without inheriting
    /// stale in-process handles from the previous run.
    pub async fn shutdown_application_loops(&self, application_id: &ApplicationId) {
        if let Some(flag) = self
            .plan_loop_shutdowns
            .write()
            .await
            .remove(application_id)
        {
            flag.store(true, Ordering::SeqCst);
            tracing::info!(
                application_id = %application_id.0,
                "PlanLoop local controller shutdown signalled by runtime host"
            );
        }
        self.plan_loop_wakers.write().await.remove(application_id);

        let worker_shutdown_count = if let Some(flags) = self
            .worker_loop_shutdowns
            .write()
            .await
            .remove(application_id)
        {
            let count = flags.len();
            for flag in flags {
                flag.store(true, Ordering::SeqCst);
            }
            count
        } else {
            0
        };
        self.worker_loop_wakers.write().await.remove(application_id);

        tracing::info!(
            application_id = %application_id.0,
            worker_shutdown_count,
            "Session-loop local runtime handles removed for application"
        );
    }
}
