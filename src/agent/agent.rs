use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{sync::Arc, time::SystemTime};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::agent::{
    AgentRegistry, AgentRuntime, AgentTools, Artifact, RuntimeEvent, SystemEvent, Task, TaskEvent,
    TaskResult, TaskStatus, WorkspaceContext,
};

#[derive(Debug, serde::Deserialize)]
pub struct LlmMode {
    pub mode: String,
}

enum AgentMode {
    Chat,
    Tool,
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

            "finish" => Ok(Self::Finish {
                message: v.message.unwrap_or_default(),
            }),

            other => Err(anyhow::anyhow!("unknown action: {}", other)),
        }
    }
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
        let mut ctx = AgentContext::new(task.prompt.clone());
        let _ = event_tx.send(RuntimeEvent::Agent(AgentEvent::Thinking {
            task: task.clone(),
        }));
        let mode = decide_mode(&ctx).await?;
        if matches!(mode, AgentMode::Chat) {
            let response = generate_chat_response(&ctx).await?;
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
            let action = decide_next_action(&ctx).await?;

            match action {
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
}

async fn decide_next_action(ctx: &AgentContext) -> anyhow::Result<AgentAction> {
    let prompt = build_prompt(ctx);
    let raw: LlmAction = prompt_ollama_for_json(&prompt).await?;
    let action = AgentAction::try_from(raw)?;
    Ok(action)
}

pub fn build_prompt(ctx: &AgentContext) -> String {
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

pub async fn prompt_ollama_for_json(prompt: &str) -> anyhow::Result<LlmAction> {
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

pub async fn prompt_ollama(input: &str) -> anyhow::Result<AgentCommand> {
    let client = Client::new();
    let url = "http://localhost:11434/api/generate";

    let system_instructions = r#"
        You are a JSON-only API. Respond ONLY with a valid JSON object matching one of these:
        {"type": "WriteFile", "data": {"path": "...", "content": "..."}}
        {"type": "ReadFile", "data": {"path": "..."}}
        {"type": "Chat", "data": {"message": "..."}}
    "#;

    let payload = json!({
        "model": "qwen3:8b",
        "prompt": format!("{}\nUser Request: {}", system_instructions, input),
        "stream": false,
        "format": "json"
    });

    let res = client
        .post(url)
        .json(&payload)
        .send()
        .await?
        .json::<OllamaResponse>()
        .await?;
    let command: AgentCommand = serde_json::from_str(&res.response)?;

    Ok(command)
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub inode: String,
    pub content: String,
    /// Human-readable location
    pub path: String,
    pub name: String,
    pub extension: Option<String>,
    pub size: u64,
    pub language: Option<String>,
    pub is_directory: bool,
    pub modified_at: Option<SystemTime>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    done: bool,
    model: String,
    response: String,
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

pub async fn decide_mode(ctx: &AgentContext) -> anyhow::Result<AgentMode> {
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

pub async fn generate_chat_response(ctx: &AgentContext) -> anyhow::Result<String> {
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

    let res = prompt_ollama_for_text(&prompt).await?;
    Ok(res)
}

pub async fn prompt_ollama_for_text(prompt: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "model": "qwen3:8b",
        "system": "You are a helpful assistant.",
        "prompt": prompt,
        "stream": false
        // ❌ NO "format": "json"
    });

    let res = client
        .post("http://localhost:11434/api/generate")
        .json(&payload)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let text = res["response"].as_str().unwrap_or("").to_string();

    Ok(text)
}
pub async fn prompt_ollama_json<T>(prompt: &str) -> anyhow::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "model": "qwen3:8b",
        "system": "Return ONLY valid JSON.",
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

    let parsed: T = serde_json::from_str(response_text)?;

    Ok(parsed)
}
