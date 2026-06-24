use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

pub enum AgentStatus {
    Thinking,
    Working(String),
    Finished(String),
    Error(String),
}

#[derive(Deserialize, Debug)]
struct OllamaResponse {
    done: bool,
    model: String,
    response: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum AgentCommand {
    WriteFile { path: String, content: String },
    ReadFile { path: String },
    Chat { message: String },
}

pub static WRITE_PROMPT: &str = r#"You are an AI assistant with file system access.
        If the user wants to save, create, or update a file, return: 
        {"type": "WriteFile", "data": {"path": "./allowed_dir/output.txt", "content": "FILE_CONTENT"}}
        Otherwise, return:
        {"type": "Chat", "data": {"message": "Your response here"}}"#;

pub static SYSTEM_PROMPT: &str = r#"
        You are an intelligent file system assistant. You must always respond with a valid JSON object that matches one of these structures:

        1. To write a file:
        {"type": "WriteFile", "data": {"path": "...", "content": "..."}}

        2. To read a file:
        {"type": "ReadFile", "data": {"path": "..."}}

        3. To communicate:
        {"type": "Chat", "data": {"message": "..."}}

        Rules:
        - Do not include any text outside the JSON object.
        - Ensure all paths are strings.
        - Escape newlines and quotes correctly within the "content" or "message" fields.
        "#;

pub async fn run_agent_loop(
    user_input: String,
    tx: UnboundedSender<AgentStatus>,
) -> anyhow::Result<()> {
    let _ = tx.send(AgentStatus::Thinking);
    let command = prompt_ollama_for_json(&user_input).await?;
    match command {
        AgentCommand::WriteFile { path, content } => {
            let _ = tx.send(AgentStatus::Working("Writing file...".to_string()));
            let target = std::path::PathBuf::from("./allowed_dir/output.txt");
            if let Some(parent) = target.parent()
                && let Err(e) = std::fs::create_dir_all(parent)
            {
                eprintln!("Failed to create directory: {}", e);
                return Err(e.into());
            }
            let _ = tx.send(AgentStatus::Finished(
                "File written successfully.".to_string(),
            ));

            match std::fs::write(&target, content) {
                Ok(_) => {
                    let msg = format!("Successfully wrote to {:?}", target);
                    let _ = tx.send(AgentStatus::Finished(msg));
                    println!("Successfully wrote to {:?}", target)
                }
                Err(e) => eprintln!("Failed to write file: {}", e),
            }
        }
        AgentCommand::Chat { message } => {
            let _ = tx.send(AgentStatus::Finished(message));
        }
        _ => {
            todo!("hi run_agent_loop");
        }
    }
    Ok(())
}

pub async fn prompt_ollama_for_json(user_input: &str) -> anyhow::Result<AgentCommand> {
    let client = reqwest::Client::new();

    let system_prompt = r#"You are an AI assistant with file system access.
        If the user wants to save, create, or update a file, return: 
        {"type": "WriteFile", "data": {"path": "./allowed_dir/output.txt", "content": "FILE_CONTENT"}}
        Otherwise, return:
        {"type": "Chat", "data": {"message": "Your response here"}}"#;

    let payload = serde_json::json!({
        "model": "qwen3:8b",
        "system": system_prompt,
        "prompt": user_input,
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
    let cmd: AgentCommand = serde_json::from_str(response_text)?;

    Ok(cmd)
}

pub async fn prompt_ollama(user_input: &str) -> anyhow::Result<AgentCommand> {
    use reqwest::Client;
    use serde_json::json;

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
        "prompt": format!("{}\nUser Request: {}", system_instructions, user_input),
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
