//! Task Router - Routes tasks to the best available agent based on capabilities.

use std::sync::Arc;

use tokio::sync::RwLock;

use super::{AgentInfo, DelegatedTask, RoutingDecision, TaskId};

/// Default confidence threshold for routing decisions.
const DEFAULT_CONFIDENCE_THRESHOLD: f32 = 0.3;

/// Router that matches tasks to agents based on capabilities.
pub struct TaskRouter {
    /// Agent registry for looking up capabilities.
    agents: Arc<RwLock<Vec<AgentInfo>>>,
    /// Minimum confidence threshold.
    confidence_threshold: f32,
}

impl TaskRouter {
    /// Create a new router.
    pub fn new(agents: Arc<RwLock<Vec<AgentInfo>>>) -> Self {
        Self {
            agents,
            confidence_threshold: DEFAULT_CONFIDENCE_THRESHOLD,
        }
    }

    /// Create with custom confidence threshold.
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold;
        self
    }

    /// Route a task to the best available agent.
    ///
    /// If `to_agent` is explicitly specified in the task, use that agent.
    /// Otherwise, analyze the prompt and match against agent capabilities.
    pub async fn route(&self, task: &DelegatedTask) -> RoutingDecision {
        // If target is explicitly specified, use it directly
        if !task.to_agent.is_empty() {
            let agents = self.agents.read().await;

            // Check if the specified agent exists
            if let Some(agent) = agents.iter().find(|a| a.name == task.to_agent) {
                if agent.available {
                    return RoutingDecision {
                        agent_name: agent.name.clone(),
                        confidence: 1.0,
                        reasoning: format!("Explicitly specified agent: {}", task.to_agent),
                        fallback_agents: self.get_fallback_agents(&agents, &task.to_agent),
                    };
                } else {
                    // Agent exists but is not available
                    return RoutingDecision {
                        agent_name: task.to_agent.clone(),
                        confidence: 0.0,
                        reasoning: format!("Agent {} is not available", task.to_agent),
                        fallback_agents: agents.iter().filter(|a| a.available).map(|a| a.name.clone()).collect(),
                    };
                }
            } else {
                // Agent doesn't exist
                return RoutingDecision {
                    agent_name: task.to_agent.clone(),
                    confidence: 0.0,
                    reasoning: format!("Agent {} not found", task.to_agent),
                    fallback_agents: agents.iter().map(|a| a.name.clone()).collect(),
                };
            }
        }

        // Analyze prompt and match against capabilities
        self.route_by_capability(task).await
    }

    /// Route based on capability matching.
    async fn route_by_capability(&self, task: &DelegatedTask) -> RoutingDecision {
        let agents = self.agents.read().await;
        let prompt_lower = task.prompt.to_lowercase();

        // Score each available agent
        let mut scored: Vec<(f32, &AgentInfo)> = agents
            .iter()
            .filter(|a| a.available)
            .map(|agent| {
                let score = self.calculate_score(&prompt_lower, agent);
                (score, agent)
            })
            .filter(|(score, _)| *score >= self.confidence_threshold)
            .collect();

        // Sort by score (highest first)
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((score, agent)) = scored.first() {
            RoutingDecision {
                agent_name: agent.name.clone(),
                confidence: *score,
                reasoning: self.generate_reasoning(&prompt_lower, agent),
                fallback_agents: scored.iter().skip(1).map(|(_, a)| a.name.clone()).collect(),
            }
        } else {
            // No match found, use coordinator as fallback
            RoutingDecision {
                agent_name: "coordinator".to_string(),
                confidence: 0.0,
                reasoning: "No matching agent found, routing to coordinator".to_string(),
                fallback_agents: agents.iter().map(|a| a.name.clone()).collect(),
            }
        }
    }

    /// Calculate a matching score for an agent based on prompt content.
    fn calculate_score(&self, prompt: &str, agent: &AgentInfo) -> f32 {
        let mut score: f32 = 0.0;

        // Check each capability for keyword matches
        for cap in &agent.capabilities {
            let cap_lower = cap.to_lowercase();

            // Direct keyword matches (highest weight)
            if prompt.contains(&cap_lower) {
                score += 0.5;
            }

            // Partial matches
            for word in prompt.split_whitespace() {
                let word_lower = word.to_lowercase();
                if cap_lower.contains(&word_lower) || word_lower.len() > 3 && cap_lower.contains(&word_lower[..word_lower.len()-1]) {
                    score += 0.2;
                }
            }
        }

        // Normalize to 0-1 range
        score.min(1.0)
    }

    /// Generate reasoning for the routing decision.
    fn generate_reasoning(&self, prompt: &str, agent: &AgentInfo) -> String {
        let matched_caps: Vec<&str> = agent
            .capabilities
            .iter()
            .filter(|cap| prompt.contains(&cap.to_lowercase()))
            .map(|s| s.as_str())
            .collect();

        if matched_caps.is_empty() {
            format!(
                "Selected {} (load: {}/{})",
                agent.name, agent.current_load, agent.max_load
            )
        } else {
            format!(
                "Matched capabilities: {} for task: {}",
                matched_caps.join(", "),
                prompt.chars().take(50).collect::<String>()
            )
        }
    }

    /// Get fallback agents (excluding the primary).
    fn get_fallback_agents(&self, agents: &[AgentInfo], exclude: &str) -> Vec<String> {
        agents
            .iter()
            .filter(|a| a.available && a.name != exclude)
            .map(|a| a.name.clone())
            .collect()
    }

    /// Update the agent registry.
    pub async fn update_agents(&self, agents: Vec<AgentInfo>) {
        let mut regs = self.agents.write().await;
        *regs = agents;
    }

    /// Add a single agent to the registry.
    pub async fn register_agent(&self, agent: AgentInfo) {
        let mut regs = self.agents.write().await;

        // Remove existing agent with same name
        regs.retain(|a| a.name != agent.name);
        regs.push(agent);
    }

    /// Remove an agent from the registry.
    pub async fn unregister_agent(&self, name: &str) {
        let mut regs = self.agents.write().await;
        regs.retain(|a| a.name != name);
    }

    /// Get all registered agents.
    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        self.agents.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_agents() -> Vec<AgentInfo> {
        vec![
            AgentInfo {
                id: "1".into(),
                name: "backend".into(),
                capabilities: vec!["backend_development".into(), "api_design".into(), "database".into()],
                current_load: 0,
                max_load: 3,
                available: true,
            },
            AgentInfo {
                id: "2".into(),
                name: "frontend".into(),
                capabilities: vec!["frontend_development".into(), "ui_design".into(), "react".into()],
                current_load: 0,
                max_load: 3,
                available: true,
            },
            AgentInfo {
                id: "3".into(),
                name: "architect".into(),
                capabilities: vec!["system_design".into(), "architecture".into()],
                current_load: 0,
                max_load: 2,
                available: true,
            },
        ]
    }

    #[tokio::test]
    async fn test_explicit_routing() {
        let agents = Arc::new(RwLock::new(create_test_agents()));
        let router = TaskRouter::new(agents);

        let task = DelegatedTask {
            id: TaskId::new(),
            from_agent: "coordinator".into(),
            to_agent: "backend".into(),
            prompt: "Write a Rust API".into(),
            priority: 5,
            parallel: false,
            created_at: chrono::Utc::now(),
            deadline: None,
            parent_task: None,
            context: None,
        };

        let decision = router.route(&task).await;
        assert_eq!(decision.agent_name, "backend");
        assert_eq!(decision.confidence, 1.0);
    }

    #[tokio::test]
    async fn test_capability_matching() {
        let agents = Arc::new(RwLock::new(create_test_agents()));
        let router = TaskRouter::new(agents);

        let task = DelegatedTask {
            id: TaskId::new(),
            from_agent: "coordinator".into(),
            to_agent: "".into(), // Let router decide
            prompt: "Create a React component with TypeScript".into(),
            priority: 5,
            parallel: false,
            created_at: chrono::Utc::now(),
            deadline: None,
            parent_task: None,
            context: None,
        };

        let decision = router.route(&task).await;
        // Should match frontend due to "react" and "typescript" keywords
        assert!(decision.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_register_agent() {
        let agents = Arc::new(RwLock::new(vec![]));
        let router = TaskRouter::new(agents.clone());

        router.register_agent(AgentInfo {
            id: "new".into(),
            name: "new_agent".into(),
            capabilities: vec!["new_capability".into()],
            current_load: 0,
            max_load: 1,
            available: true,
        }).await;

        let list = router.list_agents().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "new_agent");
    }
}
