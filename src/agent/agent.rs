use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use vfs::fs;

use crate::agent::{
    AgentEvent, AgentTools, Artifact, RuntimeEvent, Task, TaskResult, TaskStatus, WorkspaceContext,
};

#[derive(Debug, serde::Deserialize)]
pub struct LlmMode {
    pub mode: String,
}

#[derive(Debug)]
pub enum AgentMode {
    Chat,
    Tool,
}

#[derive(PartialEq, Clone)]
pub enum AgentStatus {
    Done,
    Waiting,
    Thinking,
    Error(String),
}

#[derive(Debug, serde::Deserialize)]
pub struct LlmAction {
    pub action: String,
    pub path: Option<String>,
    pub content: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug)]
pub struct AgentContext {
    pub prompt: String,
    pub workspace: WorkspaceContext,
    pub history: Vec<AgentObservation>,
    pub artifacts: Vec<Artifact>,
    pub logs: Vec<String>,
    pub spawned_tasks: Vec<Task>,
}

impl AgentContext {
    pub fn new(user_prompt: String) -> Self {
        Self {
            prompt: user_prompt,
            logs: vec![],
            artifacts: vec![],
            spawned_tasks: vec![],
            history: vec![],
            workspace: WorkspaceContext::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum AgentObservation {
    ReadFile { path: String, content: String },
    WriteFile { path: String, success: bool },
    Current { message: String },
}

#[derive(Clone, Debug)]
pub struct AgentBus {
    pub tx: UnboundedSender<AgentEvent>,
    pub event_tx: UnboundedSender<RuntimeEvent>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum AgentCommand {
    WriteFile { path: String, content: String },
    ReadFile { path: String },
    Chat { message: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "action")]
pub enum AgentAction {
    #[serde(rename = "read_file")]
    ReadFile { path: String },

    #[serde(rename = "write_file")]
    WriteFile { path: String, content: String },

    #[serde(rename = "finish")]
    Finish { message: String },

    #[serde(rename = "current")]
    Current { message: String },
}

impl TryFrom<LlmAction> for AgentAction {
    type Error = anyhow::Error;

    fn try_from(v: LlmAction) -> Result<Self, Self::Error> {
        match v.action.as_str() {
            "read_file" => Ok(Self::ReadFile {
                path: v.path.ok_or_else(|| anyhow::anyhow!("missing path"))?,
            }),

            "write_file" => Ok(Self::WriteFile {
                path: v.path.ok_or_else(|| anyhow::anyhow!("missing path"))?,
                content: v
                    .content
                    .ok_or_else(|| anyhow::anyhow!("missing content"))?,
            }),

            "current" => Ok(Self::Current {
                message: v.message.unwrap_or_default(),
            }),

            "finish" => Ok(Self::Finish {
                message: v.message.unwrap_or_default(),
            }),

            other => Err(anyhow::anyhow!("unknown action: {}", other)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Agent {
    pub id: String,
    pub tools: AgentTools,
    pub workspace: Arc<WorkspaceContext>,
}

impl Agent {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            tools: AgentTools::default(),
            workspace: Arc::new(WorkspaceContext::default()),
        }
    }
}

impl Default for Agent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent {
    pub async fn run_agent_loop(
        &self,
        task: Task,
        event_tx: UnboundedSender<RuntimeEvent>,
    ) -> anyhow::Result<TaskResult> {
        let mut steps = 0;
        let max_steps = 10;
        let mut ctx = AgentContext::new(task.prompt.clone());
        let _ = event_tx.send(RuntimeEvent::Agent(AgentEvent::Thinking {
            task: task.clone(),
        }));
        let mode: AgentMode = self.decide_mode(&ctx).await?;
        if matches!(mode, AgentMode::Chat) {
            let response = prompt_chat(&ctx).await?;
            let result = TaskResult {
                task_id: task.id.clone(),
                status: TaskStatus::Completed,
                summary: None,
                artifacts: vec![],
                logs: vec![],
                spawned_tasks: vec![],
                chat: Some(response),
            };

            let _ = event_tx.send(RuntimeEvent::Agent(AgentEvent::Finished {
                result: result.clone(),
            }));

            return Ok(result);
        }
        loop {
            steps += 1;

            if steps > max_steps {
                return Ok(TaskResult {
                    task_id: task.id,
                    status: TaskStatus::Failed("Infinite loop".to_string()),
                    summary: Some("Agent exceeded maximum reasoning steps".into()),
                    artifacts: ctx.artifacts,
                    logs: ctx.logs,
                    spawned_tasks: ctx.spawned_tasks,
                    chat: None,
                });
            }
            let action = self.decide_next_action(&ctx).await?;

            match action {
                AgentAction::Current { message } => {
                    let now = chrono::Local::now().format("%Y-%m-%d").to_string();

                    let response =
                        format!("Context update: The current date is {}. {}", now, message);

                    let _ = event_tx.send(RuntimeEvent::Agent(AgentEvent::Working {
                        task: task.clone(),
                        message: response.clone(),
                    }));

                    ctx.history
                        .push(AgentObservation::Current { message: response });
                }

                AgentAction::ReadFile { path } => {
                    let _ = event_tx.send(RuntimeEvent::Agent(AgentEvent::Working {
                        task: task.clone(),
                        message: format!("Reading {path}"),
                    }));

                    let content = self.tools.fs.read(&path)?;

                    ctx.history
                        .push(AgentObservation::ReadFile { path, content });
                }

                AgentAction::WriteFile { path, content } => {
                    let _ = event_tx.send(RuntimeEvent::Agent(AgentEvent::Working {
                        task: task.clone(),
                        message: format!("Writing {path}"),
                    }));

                    self.tools.fs.write(&path, &content)?;

                    ctx.history.push(AgentObservation::WriteFile {
                        path,
                        success: true,
                    });
                }
                AgentAction::Finish { message } => {
                    let result = TaskResult {
                        chat: None,
                        task_id: task.id,
                        status: TaskStatus::Completed,
                        summary: Some(message),
                        artifacts: ctx.artifacts,
                        logs: ctx.logs,
                        spawned_tasks: ctx.spawned_tasks,
                    };
                    let _ = event_tx.send(RuntimeEvent::Agent(AgentEvent::Finished {
                        result: result.clone(),
                    }));

                    return Ok(result);
                }
            }
        }
    }
    async fn decide_mode(&self, ctx: &AgentContext) -> anyhow::Result<AgentMode> {
        let prompt = format!(
            r#"
            You are a router.

            Decide if this request needs tools or is chat only.

            USER:
            {}

            Return JSON:
            {{
                "mode": "chat" | "tool"
            }}
            "#,
            ctx.prompt
        );

        let raw: LlmMode = prompt_ollama_json(&prompt).await?;

        Ok(match raw.mode.as_str() {
            "tool" => AgentMode::Tool,
            _ => AgentMode::Chat,
        })
    }

    async fn decide_next_action(&self, ctx: &AgentContext) -> anyhow::Result<AgentAction> {
        let prompt = build_prompt(ctx);
        let raw = build_action(&prompt).await?;
        let action = AgentAction::try_from(raw)?;
        Ok(action)
    }
}

fn build_prompt(ctx: &AgentContext) -> String {
    format!(
        r#"
            You are an agent that MUST output a single JSON object.

            Your job is to choose the next action based on the context.

            ---

            USER REQUEST:
            {}

            ---

            WORKSPACE:
            {}

            ---

            HISTORY:
            {}

            ---

            RULES:
            - Output ONLY valid JSON
            - No markdown
            - No explanation
            - No extra keys

            ---

            YOU MUST OUTPUT ONE OF THESE FORMS:

            1. Read file:
            {{
                "action": "read_file",
                "path": "relative/file/path.rs"
            }}

            2. Write file:
            {{
                "action": "write_file",
                "path": "relative/file/path.rs",
                "content": "file content here"
            }}

            3. Finish:
            {{
                "action": "finish",
                "message": "done"
            }}
        "#,
        ctx.prompt,
        format_workspace(&ctx.workspace),
        format_history(&ctx.history)
    )
}

async fn build_action(prompt: &str) -> anyhow::Result<LlmAction> {
    let client = reqwest::Client::new();

    let system_prompt = r#"
        You are a strict JSON generator.

        You must output ONLY valid JSON.
        No markdown.
        No explanation.
        No extra text.
        "#;

    let payload = serde_json::json!({
        "model": "qwen3:8b",
        "system": system_prompt,
        "prompt": prompt,
        "stream": false,
        "format": "json"
    });

    let res = client
        .post("http://localhost:11434/api/generate")
        .json(&payload)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let response_text = res["response"].as_str().unwrap_or("{}");

    let raw: LlmAction = serde_json::from_str(response_text)?;

    Ok(raw)
}

fn format_workspace(workspace: &WorkspaceContext) -> String {
    let mut output = String::new();

    for file in &workspace.files {
        output.push_str(&format!("\n--- {} ---\n{}\n", file.path, file.content));
    }

    output
}
fn format_history(history: &[AgentObservation]) -> String {
    serde_json::to_string_pretty(history).unwrap_or_else(|_| "[]".to_string())
}

pub async fn prompt_chat(ctx: &AgentContext) -> anyhow::Result<String> {
    let prompt = format!(
        r#"
            You are a helpful assistant.

            User request:
            {}

            History:
            {}

            Respond normally. No JSON. Just text.
            "#,
        ctx.prompt,
        format_history(&ctx.history)
    );

    let result = ollama_generate(&prompt, None, false).await?;
    Ok(result)
}

pub async fn prompt_ollama_json<T>(prompt: &str) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    let result = ollama_generate(prompt, Some("You are a helpful assistant"), true).await?;
    Ok(serde_json::from_str(&result)?)
}

pub async fn ollama_generate(
    prompt: &str,
    system: Option<&str>,
    json: bool,
) -> anyhow::Result<String> {
    let client = reqwest::Client::new();

    let mut payload = serde_json::json!({
        "model": "qwen3:8b",
        "prompt": prompt,
        "stream": false,
    });

    if let Some(sys_msg) = system {
        payload["system"] = serde_json::json!(sys_msg);
    }

    if json {
        payload["format"] = serde_json::json!("json");
    }

    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&payload)
        .send()
        .await?;

    let res: serde_json::Value = response.json().await?;

    res["response"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Failed to parse response field from Ollama"))
}
