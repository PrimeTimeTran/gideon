use std::env;
use std::process::Stdio; // Only if you need the constants
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

// use tokio::process::Command;
// use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};
// use std::process::Stdio;

pub async fn run_my_ui() -> Result<(), Box<dyn std::error::Error>> {
    let exe_path = env::current_exe()?;
    let mut child = Command::new(exe_path)
        .arg("start")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("Failed to open stdout"));

    // 2. Send the Request
    // Note: JSON-RPC requires a newline to signify the end of the message
    let list_request = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
    stdin
        .write_all(format!("{}\n", list_request).as_bytes())
        .await?;

    // 3. Read the response
    let mut response = String::new();
    stdout.read_line(&mut response).await?;

    // 4. Output the result
    eprintln!("🏁[GIDEON SERVER STARTED]");

    Ok(())
}
// pub async fn run_my_ui() -> Result<(), Box<dyn std::error::Error>> {
//     // 1. Launch the server process
//     let mut child = Command::new("gideon")
//         .arg("start")
//         .stdin(Stdio::piped())
//         .stdout(Stdio::piped())
//         .spawn()
//         .expect("Failed to spawn gideon server");

//     let mut stdin = child.stdin.take().expect("Failed to open stdin");
//     let mut stdout = BufReader::new(child.stdout.take().expect("Failed to open stdout"));

//     // 2. Send the Request
//     // Note: JSON-RPC requires a newline to signify the end of the message
//     let list_request = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
//     stdin.write_all(format!("{}\n", list_request).as_bytes()).await?;

//     // 3. Read the response
//     let mut response = String::new();
//     stdout.read_line(&mut response).await?;

//     // 4. Output the result
//     println!("📡 Server responded: {}", response);

//     Ok(())
// }
