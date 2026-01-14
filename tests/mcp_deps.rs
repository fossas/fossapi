//! Tests that MCP dependencies are properly available.

use rmcp::handler::server::ServerHandler;
use schemars::JsonSchema;

/// Compile-time verification that rmcp::ServerHandler is importable.
#[allow(dead_code)]
fn assert_server_handler_exists<T: ServerHandler>() {}

/// Compile-time verification that schemars::JsonSchema is importable.
#[derive(JsonSchema)]
#[allow(dead_code)]
struct TestSchema {
    field: String,
}

#[test]
fn mcp_dependencies_available() {
    // This test verifies that the MCP dependencies compile.
    // The actual verification happens at compile time via the imports above.
}
