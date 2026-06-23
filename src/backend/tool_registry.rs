use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub route: String,
    pub file: String,
}

#[derive(Debug, Default)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
}

pub fn load_tools(root: &Path) -> ToolRegistry {
    let tools_dir = root.join(".ai").join("tools");

    println!("🧰 loading tools from: {}", tools_dir.display());

    let mut registry = ToolRegistry::default();

    if !tools_dir.exists() {
        println!("⚠️ no tools directory found");
        return registry;
    }

    for entry in fs::read_dir(&tools_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }

        if let Ok(content) = fs::read_to_string(&path)
            && let Some(tool) = parse_tool(&content, &path)
        {
            println!("  🔧 registered tool: {}", tool.name);
            registry.tools.insert(tool.name.clone(), tool);
        }
    }

    println!("🧰 total tools loaded: {}", registry.tools.len());

    registry
}

pub fn parse_tool(content: &str, path: &Path) -> Option<ToolDefinition> {
    let mut name = None;
    let mut route = None;

    for line in content.lines() {
        let line = line.trim();

        if let Some(stripped) = line.strip_prefix("# Tool:") {
            name = Some(stripped.trim().to_string());
        }

        if let Some(stripped) = line.strip_prefix("Route:") {
            route = Some(stripped.trim().to_string());
        }
    }

    Some(ToolDefinition {
        name: name?,
        route: route.unwrap_or("default".to_string()),
        file: path.display().to_string(),
    })
}
