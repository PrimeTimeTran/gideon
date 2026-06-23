use std::path::Path;

use cli::Context;

pub fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

pub fn load_root_client_context(root: &Path) {
    let path = root.join(".ai").join("agents.md");
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let title = extract_ai_title(&content);
            println!("  🗂️ [CONTEXT WORKSPACE TITLE] {}", title);
            println!("  🗂️ [CONTEXT WORKSPACE PATH] {}", rel(root, &path));
        }
        Err(_) => {
            println!("⚠️ no workspace .ai/agents.md found");
        }
    }
}

pub fn load_crate_ai_context(root: &Path, crate_path: &Path) {
    let path = crate_path.join(".ai").join("agents.md");

    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let title = extract_ai_title(&content);
            println!("    📄 [CRATE ENTRY TITLE] {}", title);
            println!("    📄 [CRATE ENTRY PATH] {}", rel(root, &path));
        }
        Err(_) => {
            println!("    ⚠️ crate AI not found");
        }
    }
}

pub fn resolve_active_crates(_root: &Path, _ctx: &Context) -> Vec<String> {
    vec!["gideon".to_string()]
}

pub fn extract_ai_title(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("# ") {
            return trimmed.trim_start_matches("# ").to_string();
        }
    }

    "Untitled AI Context".to_string()
}

pub fn bootstrap_runtime(root: &Path) {
    println!("⚙️ bootstrapping runtime from {}", root.display());
}
