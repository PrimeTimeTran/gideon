#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub prompt: String,
}

#[derive(Debug, Clone)]
pub enum TaskStatus {
    Completed,
    Failed(String),
    Interrupted,
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

#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub summary: Option<String>,
    pub artifacts: Vec<Artifact>,
    pub logs: Vec<String>,
    pub spawned_tasks: Vec<Task>,
    pub chat: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Artifact {
    FileRead { path: String, content: String },
    FileWrite { path: String },
    Observation(String),
    ToolOutput(String),
}
