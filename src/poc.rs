use anyhow::Ok;
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{PromptMessage, PromptMessageContent},
};

use crate::backend::server::{CodeReviewArgs, MyServer};

// // #[tokio::main]
// pub async fn poc() -> anyhow::Result<()> {
//     let server = MyServer;
//     let hello = server.hello().await;
//     let msg = server.greeting().await;
//     let result = server
//         .code_review(rmcp::handler::server::wrapper::Parameters(
//             server::CodeReviewArgs {
//                 language: "Rust".into(),
//                 focus_areas: Some(vec!["performance".into()]),
//             },
//         ))
//         .await;
//     Ok(())
// }

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

// impl McpClient {
//     pub async fn hello(&self) -> anyhow::Result<String> {
//         Ok(self.server.hello().await)
//     }

//     // 2.1 Custom parsing.
//     // pub async fn greeting(&self) -> anyhow::Result<String> {
//     //     let msgs = self.server.greeting().await;

//     //     Ok(format_prompt_messages(msgs))
//     // }

//     // 2.2 MCP Native
//     pub async fn greeting(&self) -> anyhow::Result<Vec<PromptMessage>> {
//         Ok(self.server.greeting().await)
//     }

//     // 3.1
//     // pub async fn code_review(&self, args: CodeReviewArgs) -> anyhow::Result<GetPromptResult> {
//     //     self.server.code_review(Parameters(args)).await
//     // }
//     // 3.2
//     pub async fn code_review(&self, args: CodeReviewArgs) -> anyhow::Result<String> {
//         let res = self.server.code_review(Parameters(args)).await?;

//         Ok(format_prompt_messages(res.messages))
//     }
// }
