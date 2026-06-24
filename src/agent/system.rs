use crate::agent::{Task, TaskResult};

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
