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
        /// The type of entity to get.
        entity: Entity,

        /// The locator (projects/revisions/dependencies) or ID (issues).
        locator: String,
    },

    /// List entities with optional filtering and pagination.
    List {
        /// The type of entity to list.
        entity: Entity,

        /// Page number (1-indexed).
        #[arg(long)]
        page: Option<u32>,

        /// Number of items per page.
        #[arg(long)]
        count: Option<u32>,

        /// Revision locator (required for dependencies).
        #[arg(long)]
        revision: Option<String>,
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
