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
    #[command(
        about = "Get a snippet's details, including its matched first-party files",
        alias = "snippets"
    )]
    Snippet {
        #[arg(help = "The revision locator (e.g. custom+org/repo$ref)")]
        revision: String,
        #[arg(help = "The snippet ID")]
        snippet: String,
    },
    #[command(about = "Show the side-by-side match details for a snippet at a first-party path")]
    SnippetMatch {
        #[arg(help = "The revision locator (e.g. custom+org/repo$ref)")]
        revision: String,
        #[arg(help = "The snippet ID")]
        snippet: String,
        #[arg(help = "The first-party file path where the snippet matched")]
        path: String,
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
    #[command(
        about = "List snippets (matched OSS packages) in a revision",
        alias = "snippet"
    )]
    Snippets {
        #[arg(help = "The revision locator (e.g. custom+org/repo$ref)")]
        revision: String,
        #[arg(long, help = "Filter by file/directory path (defaults to /)")]
        path: Option<String>,
        #[arg(long)]
        page: Option<u32>,
        #[arg(long)]
        count: Option<u32>,
    },
    #[command(about = "List every snippet match location (first-party file -> matched package)")]
    SnippetLocations {
        #[arg(help = "The revision locator (e.g. custom+org/repo$ref)")]
        revision: String,
        #[arg(long, help = "Filter by file/directory path (defaults to /)")]
        path: Option<String>,
        #[arg(
            long,
            help = "Resolve the first-party line range for each match (extra API calls)"
        )]
        with_lines: bool,
    },
    #[command(about = "List the file/directory tree where snippets were detected")]
    SnippetPaths {
        #[arg(help = "The revision locator (e.g. custom+org/repo$ref)")]
        revision: String,
        #[arg(long, help = "File/directory path to drill into (defaults to /)")]
        path: Option<String>,
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
