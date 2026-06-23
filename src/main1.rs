// use std::collections::HashMap;

// use anyhow::{Error, Ok};
// use cli::Context;
// use colored::*;
// use reqwest::Client;
// use rmcp::ServiceExt;

// use tokio::{
//     io::{self, AsyncBufReadExt, stdin, stdout},
//     sync::mpsc,
// };

// mod backend;
// mod cli_process;
// mod context;
// mod reg_command;
// mod router;
// mod runtime;
// mod service;

// use crate::{
//     backend::{server, server::MyServer, tool},
//     reg_command::{Cli, Command, parse},
//     router::execute,
// };

// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     let cli = parse();
//     let ctx = Context {
//         verbose: cli.verbose,
//     };
//     run_mcp_logic().await?;
//     anyhow::Ok(())
// }

// async fn run_mcp_logic() -> anyhow::Result<()> {
//     let (tx, mut rx) = mpsc::channel::<String>(32);
//     tokio::spawn(async move {
//         let server = MyServer;
//         while let Some(command) = rx.recv().await {
//             match command.as_str() {
//                 "hello" => {
//                     let msg = server.hello().await;
//                     dbg!("Server says: {:?}", msg);
//                 }
//                 "question" => {
//                     let msg = server.hello().await;
//                     dbg!("Server says: {:?}", msg);
//                 }
//                 "review" => {
//                     let result = server
//                         .code_review(rmcp::handler::server::wrapper::Parameters(
//                             server::CodeReviewArgs {
//                                 language: "Rust".into(),
//                                 focus_areas: Some(vec!["performance".into()]),
//                             },
//                         ))
//                         .await;
//                     dbg!("Review result: {:?}", result);
//                 }
//                 _ => {
//                     let result: Result<String, Error> = prompt_ollama(command).await;

//                     if let std::result::Result::Ok(answer) = result {
//                         print_ai(&answer);
//                     } else {
//                         eprintln!("Error occurred!");
//                     }
//                 }
//             }
//         }
//     });

//     println!("MCP Shell Started. Type 'hello', 'review', or 'exit'.");

//     let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
//     let mut line = String::new();

//     loop {
//         line.clear();
//         reader.read_line(&mut line).await?;
//         let input = line.trim().to_string();

//         if input == "exit" {
//             break;
//         }

//         let _ = tx.send(input).await;
//     }

//     Ok(())
// }

// async fn prompt_ollama(user_input: String) -> anyhow::Result<String> {
//     let client = Client::new();
//     let url = "http://localhost:11434/api/generate";

//     let payload = json!({
//         "model": "qwen3:8b",
//         "prompt": format!("{} Respond with only the direct answer.", user_input),
//         "stream": false
//     });

//     let res = client
//         .post(url)
//         .json(&payload)
//         .send()
//         .await?
//         .json::<OllamaResponse>()
//         .await?;

//     anyhow::Ok(res.response.trim().to_string())
// }

// use serde::Deserialize;
// use serde_json::json;

// #[derive(Deserialize, Debug)]
// struct OllamaResponse {
//     response: String,
// }

// pub fn print_user(input: &str) {
//     println!("{}: {}", "You".blue().bold(), input);
// }

// pub fn print_ai(answer: &str) {
//     println!("{}: {}", "AI".green().bold(), answer);
// }

// pub fn print_system(msg: &str) {
//     println!("{} {}", "::".yellow(), msg.italic());
// }

// enum GideonOutput {
//     User(String),
//     AI(String),
//     System(String),
//     Error(String),
// }

// impl GideonOutput {
//     fn display(&self) {
//         match self {
//             Self::User(s) => println!("{}: {}", "You".blue(), s),
//             Self::AI(s) => println!("{}: {}", "AI".green(), s),
//             Self::System(s) => println!("{} {}", "->".yellow(), s),
//             Self::Error(s) => eprintln!("{}: {}", "Error".red(), s),
//         }
//     }
// }

// // struct Dispatcher {
// //     handlers: HashMap<char, Box<dyn CommandHandler>>,
// // }

// // impl Dispatcher {
// //     fn handle(&self, input: &str) {
// //         let prefix = input.chars().next();
// //         if let Some(h) = self.handlers.get(&prefix) {
// //             h.execute(&input[1..]);
// //         }
// //     }
// // }

// // terminal.draw(|f| draw_ui(f, &app))?;
