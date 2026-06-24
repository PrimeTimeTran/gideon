use std::collections::HashMap;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::agent::{Agent, AgentEvent, RuntimeEvent, SystemEvent, Task};

#[derive(Debug)]
pub struct AgentRegistry {
    agents: HashMap<String, Agent>,
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self {
            agents: std::collections::HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct AgentRuntime {
    pub cmd_rx: UnboundedReceiver<AgentEvent>,
    pub event_tx: UnboundedSender<RuntimeEvent>,
    pub registry: AgentRegistry,
}

impl AgentRuntime {
    pub async fn spawn_agent(&self, task: Task) {
        let event_tx = self.event_tx.clone();
        tokio::spawn(async move {
            let agent = Agent::new();

            let result = agent.run_agent_loop(task.clone(), event_tx.clone()).await;

            match result {
                Ok(result) => {
                    let _ = event_tx.send(RuntimeEvent::System(SystemEvent::TaskCompleted {
                        result: result.clone(),
                    }));

                    for t in result.spawned_tasks {
                        let _ = event_tx
                            .send(RuntimeEvent::System(SystemEvent::TaskSpawned { task: t }));
                    }
                }

                Err(e) => {
                    let _ = event_tx.send(RuntimeEvent::System(SystemEvent::TaskFailed {
                        task_id: task.id.clone(),
                        error: e.to_string(),
                    }));
                }
            }
        });
    }
}
