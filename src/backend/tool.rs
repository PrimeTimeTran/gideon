use rmcp::{
    ServerHandler, handler::server::wrapper::Parameters, schemars, tool, tool_handler, tool_router,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddVars {
    pub a: i32,
    pub b: i32,
}

#[derive(Clone)]
pub struct TempTool;

#[tool_router]
impl TempTool {
    #[tool(description = "Temporary tool to add two numbers")]
    pub fn add(&self, Parameters(AddVars { a, b }): Parameters<AddVars>) -> String {
        (a + b).to_string()
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct AddParams {
    a: i32,
    b: i32,
}

#[derive(Clone)]
struct Calculator;

#[tool_router]
impl Calculator {
    #[tool(description = "Add two numbers")]
    fn add(&self, Parameters(AddParams { a, b }): Parameters<AddParams>) -> String {
        (a + b).to_string()
    }
}

#[tool_handler(
    name = "calculator",
    version = "1.0.0",
    instructions = "A simple calculator"
)]
impl ServerHandler for Calculator {}
