//! FOSSA API client library.
//!
//! A Rust library for interacting with the FOSSA REST API using a
//! trait-based architecture where each operation (Get, List, Update)
//! is defined as a trait that entity types implement.
//!
//! # Quick Start
//!
//! ```no_run
//! use fossapi::{FossaClient, Project, Dependency, Get, List};
//!
//! #[tokio::main]
//! async fn main() -> fossapi::Result<()> {
//!     // Create client from environment variables
//!     let client = FossaClient::from_env()?;
//!
//!     // Get a project by locator
//!     let project = Project::get(&client, "custom+my-org/my-project".to_string()).await?;
//!     println!("Project: {}", project.title);
//!
//!     // List all projects
//!     let projects = Project::list_all(&client, &Default::default()).await?;
//!     println!("Found {} projects", projects.len());
//!
//!     // List dependencies for a revision
//!     let deps = fossapi::get_dependencies(
//!         &client,
//!         "custom+my-org/my-project$main",
//!         Default::default(),
//!     ).await?;
//!     println!("Found {} dependencies", deps.len());
//!
//!     Ok(())
//! }
//! ```
//!
//! # Architecture
//!
//! The library is organized around three core traits:
//!
//! - [`Get`] - Fetch a single entity by ID
//! - [`List`] - Fetch paginated collections of entities
//! - [`Update`] - Modify an existing entity
//!
//! Each entity type (like [`Project`] or [`Dependency`]) implements
//! the traits that are supported by its API endpoints.
//!
//! # Configuration
//!
//! The client reads configuration from environment variables:
//!
//! - `FOSSA_API_KEY` (required) - Your FOSSA API key
//! - `FOSSA_API_URL` (optional) - Base URL (defaults to `https://app.fossa.com/api`)

mod client;
mod error;
mod models;
mod pagination;
mod traits;

// Re-export core types
pub use client::FossaClient;
pub use error::{FossaError, Result};
pub use pagination::{Page, PaginationParams};

// Re-export traits
pub use traits::{Get, List, Update};

// Re-export models
pub use models::{
    // Project types
    LatestRevision,
    Project,
    ProjectIssues,
    ProjectListQuery,
    ProjectUpdateParams,
    // Revision types
    Revision,
    RevisionIssues,
    RevisionListQuery,
    RevisionQuery,
    RevisionStats,
    RevisionStatus,
    // Dependency types
    Dependency,
    DependencyIssue,
    DependencyListQuery,
    DependencyQuery,
    IssueStatus,
    IssueType,
    LicenseInfo,
};

// Re-export convenience functions
pub use models::{get_dependencies, get_dependencies_page};
pub use models::{get_revision, get_revisions, get_revisions_page};
