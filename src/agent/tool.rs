use anyhow::{Error, Result};
use std::time::SystemTime;

use crate::poc::McpClient;

#[derive(Debug, Default, Clone)]
pub struct FileInfo {
    pub inode: String,
    pub content: String,
    pub path: String,
    pub name: String,
    pub extension: Option<String>,
    pub size: u64,
    pub language: Option<String>,
    pub is_directory: bool,
    pub modified_at: Option<SystemTime>,
}

#[derive(Debug, Default, Clone)]
pub struct AgentTools {
    pub fs: FileSystemTool,
    pub mcp: McpClient,
}

#[derive(Debug, Clone)]
pub struct Vfs;

impl Default for Vfs {
    fn default() -> Self {
        Self
    }
}

impl Vfs {
    pub fn new() -> Self {
        Self
    }

    pub fn search_files(&self, query: &str) -> Result<Vec<FileInfo>, Error> {
        dbg!("Search VFS for files matching query");

        Ok(vec![FileInfo::default()])
    }

    pub fn create_file(&self, path: &str, content: &str) -> Result<(), Error> {
        dbg!("Create file in VFS");

        println!("Created file: {} ({} bytes)", path, content.len());

        Ok(())
    }

    pub fn read_file(&self, path: &str) -> Result<String, Error> {
        dbg!("Read file contents from VFS");

        Ok(format!(
            "// Placeholder content for {}\n\nfn main() {{}}\n",
            path
        ))
    }

    pub fn write_file(&self, path: &str, content: &str) -> Result<(), Error> {
        dbg!("write_file");
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[derive(Default, Debug, Clone)]
pub struct FileSystemTool {
    pub vfs: Vfs,
}

impl FileSystemTool {
    pub fn search(&self, query: &str) -> anyhow::Result<Vec<FileInfo>> {
        self.vfs.search_files(query)
    }

    pub fn read(&self, path: &str) -> anyhow::Result<String> {
        self.vfs.read_file(path)
    }

    pub fn write(&self, path: &str, content: &str) -> anyhow::Result<()> {
        self.vfs.write_file(path, content)
    }

    pub fn create(&self, path: &str, content: &str) -> anyhow::Result<()> {
        self.vfs.create_file(path, content)
    }
}
