// use async_trait::async_trait;
// use cli::{CliCommand, Context};

// use crate::rmcp::server::start_server_service;

// pub struct Server;

// #[async_trait]
// impl CliCommand for Server {
//     async fn run(&self, ctx: &Context) {
//         println!("🏁[MCP SERVER START]");
//         start_server_service()
//             .await
//             .expect("Failed to start server service");
//         // let client_root = std::env::current_dir().unwrap();
//     }
// }

// pub struct Client;

// #[async_trait]
// impl CliCommand for Client {
//     async fn run(&self, ctx: &Context) {
//         println!("🏁[MCP CLIENT START]");
//         // let client_root = std::env::current_dir().unwrap();
//     }
// }
