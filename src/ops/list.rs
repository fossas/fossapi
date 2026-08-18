//! The `list` verb: list entities, with pagination where the API supports it.

use clap::{Args, Subcommand};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::PageArgs;
use crate::{
    get_dependencies_page, get_issues_page, get_revisions_page, get_snippet_locations_page,
    get_snippet_paths, get_snippets_page, Dependency, FossaClient, Issue, IssueCategory,
    IssueListQuery, List, Page, Project, Result, Revision, Snippet, SnippetListQuery,
    SnippetLocation, SnippetPath,
};

/// Parameters for `list projects`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct ListProjectsParams {
    /// Pagination.
    #[command(flatten)]
    #[serde(flatten)]
    pub pagination: PageArgs,
}

/// Parameters for `list issues`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct ListIssuesParams {
    /// Issue category to list.
    #[arg(long, value_enum)]
    pub category: IssueCategory,

    /// Pagination.
    #[command(flatten)]
    #[serde(flatten)]
    pub pagination: PageArgs,
}

/// Parameters for `list dependencies`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct ListDependenciesParams {
    /// The revision locator (e.g., "custom+org/repo$ref").
    pub revision: String,

    /// Pagination.
    #[command(flatten)]
    #[serde(flatten)]
    pub pagination: PageArgs,
}

/// Parameters for `list revisions`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct ListRevisionsParams {
    /// The project locator (e.g., "custom+org/repo").
    pub project: String,

    /// Pagination.
    #[command(flatten)]
    #[serde(flatten)]
    pub pagination: PageArgs,
}

/// Parameters for `list snippets`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct ListSnippetsParams {
    /// The revision locator (e.g., "custom+org/repo$ref").
    pub revision: String,

    /// Filter by file/directory path (defaults to /).
    #[arg(long)]
    pub path: Option<String>,

    /// Pagination.
    #[command(flatten)]
    #[serde(flatten)]
    pub pagination: PageArgs,
}

/// Parameters for `list snippet-locations`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct ListSnippetLocationsParams {
    /// The revision locator (e.g., "custom+org/repo$ref").
    pub revision: String,

    /// Filter by file/directory path (defaults to /).
    #[arg(long)]
    pub path: Option<String>,

    /// Resolve the first-party line range for each match (extra API calls).
    #[arg(long)]
    #[serde(default)]
    pub with_lines: bool,

    /// Pagination (over the underlying snippets; each page returns every
    /// location its snippets contain).
    #[command(flatten)]
    #[serde(flatten)]
    pub pagination: PageArgs,
}

/// Parameters for `list snippet-paths`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct ListSnippetPathsParams {
    /// The revision locator (e.g., "custom+org/repo$ref").
    pub revision: String,

    /// File/directory path to drill into (defaults to /).
    #[arg(long)]
    pub path: Option<String>,
}

/// The `list` operation, declared once for both the CLI and the MCP server.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(tag = "entity", rename_all = "snake_case")]
pub enum ListCommand {
    /// List all projects.
    #[command(alias = "project")]
    Projects(ListProjectsParams),

    /// List all issues in a category.
    #[command(alias = "issue")]
    Issues(ListIssuesParams),

    /// List dependencies for a revision.
    #[command(alias = "dependency")]
    Dependencies(ListDependenciesParams),

    /// List revisions for a project.
    #[command(alias = "revision")]
    Revisions(ListRevisionsParams),

    /// List snippets (matched OSS packages) in a revision.
    #[command(alias = "snippet")]
    Snippets(ListSnippetsParams),

    /// List every snippet match location (first-party file -> matched package).
    SnippetLocations(ListSnippetLocationsParams),

    /// List the file/directory tree where snippets were detected.
    SnippetPaths(ListSnippetPathsParams),
}

/// The result of a [`ListCommand`], serialized as the inner page or list.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ListOutput {
    /// A page of projects.
    Projects(Page<Project>),
    /// A page of issues.
    Issues(Page<Issue>),
    /// A page of dependencies.
    Dependencies(Page<Dependency>),
    /// A page of revisions.
    Revisions(Page<Revision>),
    /// A page of snippets.
    Snippets(Page<Snippet>),
    /// A page of flattened snippet match locations (paginated over snippets).
    SnippetLocations(Page<SnippetLocation>),
    /// The snippet path tree at one level (not paginated).
    SnippetPaths(Vec<SnippetPath>),
}

/// Execute a `list` operation.
pub async fn run_list(client: &FossaClient, command: ListCommand) -> Result<ListOutput> {
    Ok(match command {
        ListCommand::Projects(p) => {
            let (page, count) = p.pagination.resolve();
            ListOutput::Projects(
                Project::list_page(client, &Default::default(), page, count).await?,
            )
        }
        ListCommand::Issues(p) => {
            let (page, count) = p.pagination.resolve();
            let query = IssueListQuery {
                category: Some(p.category),
                ..Default::default()
            };
            ListOutput::Issues(get_issues_page(client, query, page, count).await?)
        }
        ListCommand::Dependencies(p) => {
            let (page, count) = p.pagination.resolve();
            ListOutput::Dependencies(
                get_dependencies_page(client, &p.revision, Default::default(), page, count).await?,
            )
        }
        ListCommand::Revisions(p) => {
            let (page, count) = p.pagination.resolve();
            ListOutput::Revisions(
                get_revisions_page(client, &p.project, Default::default(), page, count).await?,
            )
        }
        ListCommand::Snippets(p) => {
            let (page, count) = p.pagination.resolve();
            let query = SnippetListQuery {
                path: p.path,
                ..Default::default()
            };
            ListOutput::Snippets(get_snippets_page(client, &p.revision, query, page, count).await?)
        }
        ListCommand::SnippetLocations(p) => {
            let (page, count) = p.pagination.resolve();
            let query = SnippetListQuery {
                path: p.path,
                ..Default::default()
            };
            ListOutput::SnippetLocations(
                get_snippet_locations_page(client, &p.revision, query, p.with_lines, page, count)
                    .await?,
            )
        }
        ListCommand::SnippetPaths(p) => {
            let query = SnippetListQuery {
                path: p.path,
                ..Default::default()
            };
            ListOutput::SnippetPaths(get_snippet_paths(client, &p.revision, query).await?)
        }
    })
}
