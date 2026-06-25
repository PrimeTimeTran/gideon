use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::Path,
};

#[derive(Clone)]
pub struct Logger {
    session_id: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Log {
    session: u64,
    role: String,
    content: String,
    time: u64,
}

const PATH: &str = "tmp/chat_log.jsonl";

impl Logger {
    pub fn new(session_id: u64) -> Self {
        Self { session_id }
    }
    pub fn log_to_file(&self, role: &str, content: &str) -> io::Result<()> {
        let entry = Log {
            session: self.session_id,
            role: role.to_string(),
            content: content.to_string(),
            time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let mut file = OpenOptions::new().create(true).append(true).open(PATH)?;

        let json = serde_json::to_string(&entry).map_err(io::Error::other)?;
        writeln!(file, "{}", json)?;
        Ok(())
    }

    pub fn load_history(&self) -> (Vec<String>, Vec<String>) {
        if !Path::new(PATH).exists() {
            return (Vec::new(), Vec::new());
        }

        let mut user_h = Vec::new();
        let mut ai_h = Vec::new();

        let file = match File::open(PATH) {
            Ok(f) => f,
            Err(_) => return (Vec::new(), Vec::new()),
        };

        let reader = BufReader::new(file);
        for l in reader.lines().map_while(Result::ok) {
            if let Ok(entry) = serde_json::from_str::<Log>(&l) {
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
