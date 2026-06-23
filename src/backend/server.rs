use rmcp::handler::server::router::Router;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, ServiceExt,
    handler::server::{router::prompt::PromptRouter, tool::ToolRouter, wrapper::Parameters},
    model::*,
    prompt, prompt_handler, prompt_router,
    schemars::JsonSchema,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};

use crate::tool::AddVars;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CodeReviewArgs {
    #[schemars(description = "Programming language of the code")]
    pub language: String,
    #[schemars(description = "Focus areas for the review")]
    pub focus_areas: Option<Vec<String>>,
}

#[derive(Clone, Default)]
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
}

#[prompt_router]
impl MyServer {
    #[prompt(name = "greeting", description = "A simple greeting")]
    async fn greeting(&self) -> Vec<PromptMessage> {
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
// #[tool_router]
// impl MyServer {
// #[rmcp::tool(description = "Add two numbers")]
// pub fn add(&self, Parameters(args): Parameters<AddVars>) -> String {
//     (args.a + args.b).to_string()
// }
// }
// #[prompt_handler]
// impl ServerHandler for MyServer {
//     fn get_info(&self) -> ServerInfo {
//         let mut handler = ServerInfo::default();
//         handler.capabilities = ServerCapabilities::builder()
//             .enable_resources()
//             .enable_tools()
//             .build();
//         handler
//     }
//     async fn list_tools(
//         &self,
//         params: Option<PaginatedRequestParams>,
//         ctx: RequestContext<RoleServer>,
//     ) -> Result<ListToolsResult, McpError> {
//         self.tool_router.list_all();
//         return Ok(ListToolsResult::default());
//     }

//     async fn list_prompts(
//         &self,
//         params: Option<PaginatedRequestParams>,
//         ctx: RequestContext<RoleServer>,
//     ) -> Result<ListPromptsResult, McpError> {
//         self.prompt_router.list_all();
//         return Ok(ListPromptsResult::default());
//     }
// }
// #[prompt_handler]
// impl ServerHandler for MyServer {

// }
