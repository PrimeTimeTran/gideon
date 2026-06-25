use tokio::sync::mpsc::UnboundedReceiver;

use crate::agent::{
    AgentBus, AgentEvent, AgentRegistry, AgentRuntime, RuntimeEvent, Task, TaskResult,
};

#[derive(Debug, Clone)]
pub enum SystemEvent {
    SpawnAgent { agent_id: String },

    TaskAdd { task_id: String },
    TaskSpawned { task: Task },
    TaskQueued { task_id: String },

    TaskStarted { task_id: String },

    TaskCompleted { result: TaskResult },
    TaskFailed { task_id: String, error: String },

    AgentSpawned { agent_id: String },
    AgentFinished { agent_id: String },

    TaskGroupFinished { group_id: String },
    AllIdle,
}

pub fn new_agent_system() -> (AgentBus, AgentRuntime, UnboundedReceiver<RuntimeEvent>) {
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<AgentEvent>();
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<RuntimeEvent>();

    let bus = AgentBus {
        tx: cmd_tx,
        event_tx: event_tx.clone(),
    };

    let runtime = AgentRuntime {
        cmd_rx,
        event_tx,
        registry: AgentRegistry::default(),
    };

    (bus, runtime, event_rx)
}

pub async fn run_agent_manager(mut runtime: AgentRuntime) {
    while let Some(cmd) = runtime.cmd_rx.recv().await {
        handle_event(cmd, &runtime).await;
    }
}
pub async fn handle_event(event: AgentEvent, runtime: &AgentRuntime) {
    match event {
        AgentEvent::NewTask { task } => {
            runtime.spawn_agent(task).await;
        }

        AgentEvent::Finished { result } => {
            let _ = runtime
                .event_tx
                .send(RuntimeEvent::System(SystemEvent::TaskCompleted { result }));
        }

        _ => {}
    }
}
