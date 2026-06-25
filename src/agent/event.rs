use crate::agent::{SystemEvent, Task, TaskResult};

#[derive(Clone, Debug)]
pub enum RuntimeEvent {
    System(SystemEvent),
    Agent(AgentEvent),
}

#[derive(Debug, Clone)]
pub enum AgentEvent {
    NewTask { task: Task },
    Thinking { task: Task },
    Working { task: Task, message: String },
    Finished { result: TaskResult },
    TaskEvent { task: Task, event: TaskEvent },
}

#[derive(Debug, Clone)]
pub enum TaskEvent {
    Thinking,
    Started,
    Log(String),
    Working(String),
    Finished(String),
    Error(String),
}
