//! Registry of all per-application executor instances.
//!
//! Top-level container managing isolated `ApplicationExecutor` sandboxes keyed by
//! `ApplicationId`. Each registration spawns an independent worker supervisor.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::executor::ApplicationExecutor;
use super::types::ApplicationExecutorConfig;
use crate::executor::{
    AgentInfo, AgentRunner, ApplicationId, CallbackDispatcher, DelegatedTask, EventBus,
    ExecutionQueue, ExecutorCommand, ExecutorEvent, ExecutorEventFactory, ForkManager,
    RoutingDecision, SystemEvent, TaskContext, TaskId, TaskResult, TaskRouter, TaskStatus,
};

/// Registry of all application executors.
///
/// This is the top-level container that manages all isolated application
/// executors. Each application gets its own executor instance.
pub struct ApplicationExecutorRegistry {
    executors: RwLock<HashMap<ApplicationId, Arc<ApplicationExecutor>>>,
    default_runner: Arc<dyn AgentRunner>,
}

impl ApplicationExecutorRegistry {
    /// Create a new registry with a default agent runner.
    pub fn new(default_runner: Arc<dyn AgentRunner>) -> Self {
        Self {
            executors: RwLock::new(HashMap::new()),
            default_runner,
        }
    }

    /// Register a new application with its agents.
    pub async fn register_application(
        &self,
        application_id: ApplicationId,
        application_name: String,
        agents: Vec<AgentInfo>,
    ) -> Arc<ApplicationExecutor> {
        self.register_application_with_config(
            application_id,
            application_name,
            agents,
            ApplicationExecutorConfig::default(),
        )
        .await
    }

    /// Register a new application with custom configuration.
    pub async fn register_application_with_config(
        &self,
        application_id: ApplicationId,
        application_name: String,
        agents: Vec<AgentInfo>,
        config: ApplicationExecutorConfig,
    ) -> Arc<ApplicationExecutor> {
        let executor = Arc::new(ApplicationExecutor::new(
            application_id.clone(),
            application_name,
            agents,
            Arc::clone(&self.default_runner),
            config,
        ));

        self.executors
            .write()
            .await
            .insert(application_id, Arc::clone(&executor));
        executor
    }

    /// Get an executor by application ID.
    pub async fn get(&self, application_id: &ApplicationId) -> Option<Arc<ApplicationExecutor>> {
        self.executors.read().await.get(application_id).cloned()
    }

    /// Unregister an application.
    pub async fn unregister(&self, application_id: &ApplicationId) -> bool {
        if let Some(executor) = self.executors.write().await.remove(application_id) {
            executor.shutdown().await;
            true
        } else {
            false
        }
    }

    /// List all registered applications.
    pub async fn list_applications(&self) -> Vec<(ApplicationId, String)> {
        self.executors
            .read()
            .await
            .iter()
            .map(|(id, exec)| (id.clone(), exec.application_name.clone()))
            .collect()
    }
}
