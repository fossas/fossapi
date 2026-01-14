//! CLI argument parsing types.
//!
//! This module provides the command-line interface structure for the fossapi binary.

use clap::{Parser, Subcommand, ValueEnum};

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
        /// The type of entity to update.
        entity: Entity,

        /// The locator of the entity to update.
        locator: String,

        /// New title for the entity.
        #[arg(long)]
        title: Option<String>,

        /// New description for the entity.
        #[arg(long)]
        description: Option<String>,

        /// Set project visibility (true = public, false = private).
        #[arg(long)]
        public: Option<bool>,
    },

    /// Run the MCP server on stdio.
    Mcp {
        /// Enable verbose (debug) logging.
        #[arg(long)]
        verbose: bool,
    },
}

/// Subcommands for the `get` command with type-safe ID parsing.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum GetCommand {
    /// Get a project by locator.
    #[command(alias = "projects")]
    Project {
        /// The project locator (e.g., "custom+org/repo").
        locator: String,
    },
    /// Get a revision by locator.
    #[command(alias = "revisions")]
    Revision {
        /// The revision locator (e.g., "custom+org/repo$ref").
        locator: String,
    },
    /// Get an issue by numeric ID.
    #[command(alias = "issues")]
    Issue {
        /// The issue ID.
        id: u64,
    },
}

/// Subcommands for the `list` command with type-safe argument parsing.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum ListCommand {
    /// List all projects.
    #[command(alias = "project")]
    Projects {
        /// Page number (1-indexed).
        #[arg(long)]
        page: Option<u32>,

        /// Number of items per page.
        #[arg(long)]
        count: Option<u32>,
    },
    /// List all issues.
    #[command(alias = "issue")]
    Issues {
        /// Page number (1-indexed).
        #[arg(long)]
        page: Option<u32>,

        /// Number of items per page.
        #[arg(long)]
        count: Option<u32>,
    },
    /// List dependencies for a revision.
    #[command(alias = "dependency")]
    Dependencies {
        /// The revision locator (e.g., "custom+org/repo$ref").
        #[arg(long, required_unless_present = "revision_positional")]
        revision: Option<String>,

        /// The revision locator (positional, alternative to --revision).
        #[arg(index = 1, required_unless_present = "revision")]
        revision_positional: Option<String>,
    },
    /// List revisions for a project.
    #[command(alias = "revision")]
    Revisions {
        /// The project locator (e.g., "custom+org/repo").
        project: String,

        /// Page number (1-indexed).
        #[arg(long)]
        page: Option<u32>,

        /// Number of items per page.
        #[arg(long)]
        count: Option<u32>,
    },
}

/// Entity types that can be operated on.
#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum Entity {
    /// A FOSSA project.
    #[value(alias = "projects")]
    Project,
    /// A project revision.
    #[value(alias = "revisions")]
    Revision,
    /// A package dependency.
    #[value(alias = "dependencies")]
    Dependency,
    /// A security/licensing/quality issue.
    #[value(alias = "issues")]
    Issue,
}
