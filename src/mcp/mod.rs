//! MCP (Model Context Protocol) server and tool types.
//!
//! This module provides an MCP server implementation for the FOSSA API,
//! allowing AI assistants to query projects, revisions, issues, and dependencies.
//!
//! # Example
//!
//! ```no_run
//! use fossapi::mcp::FossaServer;
//!
//! # fn main() -> fossapi::Result<()> {
//! let server = FossaServer::from_env()?;
//! // Server can now be used with rmcp transport
//! # Ok(())
//! # }
//! ```

mod params;
mod server;

pub use params::*;
pub use server::FossaServer;
