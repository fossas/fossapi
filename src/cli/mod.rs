//! CLI argument parsing types.
//!
//! This module provides the command-line interface structure for the fossapi
//! binary. The per-entity subcommands and their parameters are the shared
//! operation declarations in [`crate::ops`], so the CLI and the MCP server
//! cannot drift apart.

use clap::{Parser, Subcommand};

pub use crate::ops::{GetCommand, IgnoreCommand, ListCommand, UnignoreCommand, UpdateCommand};

/// FOSSA API command-line interface.
#[derive(Parser, Debug)]
#[command(name = "fossapi", about = "FOSSA API CLI", version)]
pub struct Cli {
    /// Output results as JSON instead of a table.
    #[arg(long, global = true, default_value = "false")]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

/// Available CLI commands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Get a single entity by locator or ID.
    Get {
        #[command(subcommand)]
        command: GetCommand,
    },

    /// List entities with optional filtering and pagination.
    List {
        #[command(subcommand)]
        command: ListCommand,
    },

    /// Update an entity.
    Update {
        #[command(subcommand)]
        command: UpdateCommand,
    },

    /// Ignore an issue (with an optional comment and reason).
    Ignore {
        #[command(subcommand)]
        command: IgnoreCommand,
    },

    /// Revert a previous ignore, returning the issue to active.
    Unignore {
        #[command(subcommand)]
        command: UnignoreCommand,
    },

    /// Run the MCP server on stdio.
    Mcp {
        /// Enable verbose (debug) logging.
        #[arg(long)]
        verbose: bool,
    },
}
