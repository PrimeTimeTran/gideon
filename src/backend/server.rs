use rmcp::{
    ErrorData as McpError, handler::server::wrapper::Parameters, model::*, prompt, prompt_router,
    schemars::JsonSchema, tool, tool_router,
};
use serde::{Deserialize, Serialize};

use crate::backend::tool::AddVars;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CodeReviewArgs {
    #[schemars(description = "Programming language of the code")]
    pub language: String,
    #[schemars(description = "Focus areas for the review")]
    pub focus_areas: Option<Vec<String>>,
}

#[derive(Clone, Default, Debug)]
pub struct MyServer;

#[tool_router(server_handler)]
impl MyServer {
    #[tool(description = "Say hello")]
    pub async fn hello(&self) -> String {
        "Hello, world!".into()
    }
    pub fn get_info(&self) -> ServerInfo {
        let mut handler = ServerInfo::default();
        handler.capabilities = ServerCapabilities::builder()
            .enable_resources()
            .enable_tools()
            .build();
        handler
    }
    pub fn add(&self, Parameters(args): Parameters<AddVars>) -> String {
        (args.a + args.b).to_string()
    }
}

#[prompt_router]
impl MyServer {
    #[prompt(name = "greeting", description = "A simple greeting")]
    pub async fn greeting(&self) -> Vec<PromptMessage> {
        vec![PromptMessage::new_text(
            PromptMessageRole::User,
            "Hello! How can you help me today?",
        )]
    }
    #[prompt(name = "code_review", description = "Review code in a given language")]
    pub async fn code_review(
        &self,
        Parameters(args): Parameters<CodeReviewArgs>,
    ) -> Result<GetPromptResult, McpError> {
        let focus = args
            .focus_areas
            .unwrap_or_else(|| vec!["correctness".into()]);

        let mut messages = vec![PromptMessage::new_text(
            PromptMessageRole::User,
            format!(
                "Please review my {} code. Focus on: {}",
                args.language,
                focus.join(", ")
            ),
        )];

        let system_message = PromptMessage::new_text(
            PromptMessageRole::Assistant,
            "You are a helpful code reviewer. Provide constructive feedback.",
        );
        messages.insert(0, system_message);
        let mut result = GetPromptResult::default();
        result.messages = messages;
        Ok(result)
    }
}
