use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::Path,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct LogEntry {
    session: u64,
    role: String,
    content: String,
    timestamp: u64,
}

#[derive(Clone)]
pub struct Logger {
    session_id: u64,
}

impl Logger {
    pub fn new(session_id: u64) -> Self {
        Self { session_id }
    }
    pub fn log_to_file(&self, role: &str, content: &str) -> io::Result<()> {
        let entry = LogEntry {
            session: self.session_id,
            role: role.to_string(),
            content: content.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("chat_log.jsonl")?;

        let json = serde_json::to_string(&entry).map_err(io::Error::other)?;
        writeln!(file, "{}", json)?;
        Ok(())
    }

    pub fn load_history(&self) -> (Vec<String>, Vec<String>) {
        let path = "chat_log.jsonl";
        if !Path::new(path).exists() {
            return (Vec::new(), Vec::new());
        }

        let mut user_h = Vec::new();
        let mut ai_h = Vec::new();

        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return (Vec::new(), Vec::new()),
        };

        let reader = BufReader::new(file);
        for l in reader.lines().map_while(Result::ok) {
            if let Ok(entry) = serde_json::from_str::<LogEntry>(&l) {
                match entry.role.as_str() {
                    "user" => user_h.push(entry.content),
                    "ai" => ai_h.push(entry.content),
                    _ => {}
                }
            }
        }

        (user_h, ai_h)
    }
}
