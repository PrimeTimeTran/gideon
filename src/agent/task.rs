use crate::agent::{
    AgentContext
};


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
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub summary: Option<String>,
    pub artifacts: Vec<Artifact>,
    pub logs: Vec<String>,
    pub spawned_tasks: Vec<Task>,
    pub chat: Option<String>,
}

pub struct TaskContext {
    pub task_id: String,
    pub artifacts: Vec<Artifact>,
    pub logs: Vec<String>,
    pub spawned_tasks: Vec<Task>,
}

impl TaskResult {
    pub fn completed_chat(
        task_id: String,
        ctx: AgentContext,
        chat: String,
    ) -> Self {
        Self {
            task_id,
            status: TaskStatus::Completed,
            summary: None,
            artifacts: ctx.artifacts,
            logs: ctx.logs,
            spawned_tasks: ctx.spawned_tasks,
            chat: Some(chat),
        }
    }

    pub fn completed_with_summary(
        task_id: String,
        ctx: AgentContext,
        summary: String,
    ) -> Self {
        Self {
            task_id,
            status: TaskStatus::Completed,
            summary: Some(summary),
            artifacts: ctx.artifacts,
            logs: ctx.logs,
            spawned_tasks: ctx.spawned_tasks,
            chat: None,
        }
    }

    pub fn failed(
        task_id: String,
        ctx: AgentContext,
        reason: impl Into<String>,
        summary: Option<String>,
    ) -> Self {
        Self {
            task_id,
            status: TaskStatus::Failed(reason.into()),
            summary,
            artifacts: ctx.artifacts,
            logs: ctx.logs,
            spawned_tasks: ctx.spawned_tasks,
            chat: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Artifact {
    FileRead { path: String, content: String },
    FileWrite { path: String },
    Observation(String),
    ToolOutput(String),
}
