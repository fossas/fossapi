//! The `get` verb: fetch a single entity.

use clap::{Args, Subcommand};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    get_snippet_details, get_snippet_match, FossaClient, Get, Issue, IssueCategory, PrettyPrint,
    Project, Result, Revision, Snippet, SnippetMatchDetails,
};

/// Parameters for `get project`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct GetProjectParams {
    /// The project locator (e.g., "custom+org/repo").
    pub locator: String,
}

/// Parameters for `get revision`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct GetRevisionParams {
    /// The revision locator (e.g., "custom+org/repo$ref").
    pub locator: String,
}

/// Parameters for `get issue`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct GetIssueParams {
    /// The issue ID.
    pub id: u64,

    /// Issue category. Omit to search every category (up to 3 requests).
    #[arg(long, value_enum)]
    pub category: Option<IssueCategory>,
}

/// Parameters for `get snippet`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct GetSnippetParams {
    /// The revision locator (e.g., "custom+org/repo$ref").
    pub revision: String,

    /// The snippet ID.
    pub snippet: String,
}

/// Parameters for `get snippet-match`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct GetSnippetMatchParams {
    /// The revision locator (e.g., "custom+org/repo$ref").
    pub revision: String,

    /// The snippet ID.
    pub snippet: String,

    /// The first-party file path where the snippet matched.
    pub path: String,
}

/// The `get` operation, declared once for both the CLI and the MCP server.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(tag = "entity", rename_all = "snake_case")]
pub enum GetCommand {
    /// Get a project by locator.
    #[command(alias = "projects")]
    Project(GetProjectParams),

    /// Get a revision by locator.
    #[command(alias = "revisions")]
    Revision(GetRevisionParams),

    /// Get an issue by numeric ID.
    #[command(alias = "issues")]
    Issue(GetIssueParams),

    /// Get a snippet's details, including its matched first-party files.
    #[command(alias = "snippets")]
    Snippet(GetSnippetParams),

    /// Show the side-by-side match details for a snippet at a first-party path.
    SnippetMatch(GetSnippetMatchParams),
}

/// The result of a [`GetCommand`], serialized as the inner entity.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum GetOutput {
    /// A project.
    Project(Project),
    /// A revision.
    Revision(Revision),
    /// An issue.
    Issue(Box<Issue>),
    /// A snippet with its per-file matches.
    Snippet(Snippet),
    /// Side-by-side match details for a snippet at one path.
    SnippetMatch(SnippetMatchDetails),
}

impl PrettyPrint for GetOutput {
    fn pretty_print(&self) -> String {
        match self {
            GetOutput::Project(p) => p.pretty_print(),
            GetOutput::Revision(r) => r.pretty_print(),
            GetOutput::Issue(i) => i.pretty_print(),
            GetOutput::Snippet(s) => s.pretty_print(),
            GetOutput::SnippetMatch(m) => m.pretty_print(),
        }
    }
}

/// Execute a `get` operation.
pub async fn run_get(client: &FossaClient, command: GetCommand) -> Result<GetOutput> {
    Ok(match command {
        GetCommand::Project(p) => GetOutput::Project(Project::get(client, p.locator).await?),
        GetCommand::Revision(p) => GetOutput::Revision(Revision::get(client, p.locator).await?),
        GetCommand::Issue(p) => GetOutput::Issue(Box::new(match p.category {
            Some(category) => Issue::get_with_category(client, p.id, category).await?,
            None => Issue::get(client, p.id).await?,
        })),
        GetCommand::Snippet(p) => {
            GetOutput::Snippet(get_snippet_details(client, &p.revision, &p.snippet).await?)
        }
        GetCommand::SnippetMatch(p) => GetOutput::SnippetMatch(
            get_snippet_match(client, &p.revision, &p.snippet, &p.path).await?,
        ),
    })
}
