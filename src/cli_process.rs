use std::{env, process::Stdio};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

pub async fn run_my_ui() -> Result<(), Box<dyn std::error::Error>> {
    let exe_path = env::current_exe()?;
    let mut child = Command::new(exe_path)
        .arg("start")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("Failed to open stdout"));
    let list_request = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
    stdin
        .write_all(format!("{}\n", list_request).as_bytes())
        .await?;

    let mut response = String::new();
    stdout.read_line(&mut response).await?;

    eprintln!("🏁[GIDEON SERVER STARTED]");

    Ok(())
}
