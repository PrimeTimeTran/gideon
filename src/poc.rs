use anyhow::Ok;
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{PromptMessage, PromptMessageContent},
};

use crate::backend::server::{CodeReviewArgs, MyServer};

#[derive(Default, Debug, Clone)]
pub struct McpClient {
    server: MyServer,
}

impl McpClient {
    pub async fn hello(&self) -> anyhow::Result<String> {
        Ok(self.server.hello().await)
    }

    pub async fn greeting(&self) -> anyhow::Result<Vec<PromptMessage>> {
        Ok(self.server.greeting().await)
    }

    pub async fn code_review(&self, args: CodeReviewArgs) -> anyhow::Result<String> {
        let res = self.server.code_review(Parameters(args)).await?;

        Ok(format_prompt_messages(res.messages))
    }
}

fn format_prompt_messages(msgs: Vec<rmcp::model::PromptMessage>) -> String {
    msgs.into_iter()
        .map(|m| match m.content {
            PromptMessageContent::Text { text } => text,
            other => format!("{other:?}"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}
