use crate::agent::FileInfo;

#[derive(Debug, Clone)]
pub struct WorkspaceContext {
    pub files: Vec<FileInfo>,
}
impl WorkspaceContext {
    pub fn new(files: Vec<FileInfo>) -> Self {
        Self { files }
    }
}

impl Default for WorkspaceContext {
    fn default() -> Self {
        Self { files: vec![] }
    }
}
impl WorkspaceContext {
    pub fn load() -> anyhow::Result<Self> {
        // TODO:
        // load from VFS

        Ok(Self { files: vec![] })
    }
}
