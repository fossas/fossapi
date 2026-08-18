//! The `update` verb: modify an entity. Projects (metadata) and issues
//! (ignore/unignore) are updatable.

use clap::{Args, Subcommand};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    FossaClient, FossaError, Issue, IssueAction, IssueCategory, IssueIgnoreReason,
    IssueUpdateParams, PrettyPrint, Project, ProjectUpdateParams, Result, Update,
};

/// Parameters for `update project`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct UpdateProjectParams {
    /// The project locator (e.g., "custom+org/repo").
    pub locator: String,

    /// New title for the project.
    #[arg(long)]
    pub title: Option<String>,

    /// New description for the project.
    #[arg(long)]
    pub description: Option<String>,

    /// New project URL.
    #[arg(long)]
    pub url: Option<String>,

    /// Set project visibility (true = public, false = private).
    #[arg(long)]
    pub public: Option<bool>,

    /// Policy ID to apply to the project.
    #[arg(long)]
    pub policy_id: Option<u64>,

    /// Default branch name.
    #[arg(long)]
    pub default_branch: Option<String>,
}

/// Parameters for `update issue` (ignore/unignore).
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[command(group = clap::ArgGroup::new("issue_action")
    .required(true)
    .args(["ignore", "unignore"]))]
pub struct UpdateIssueParams {
    /// The issue ID.
    pub id: u64,

    /// Issue category (required for writes; the API scopes them to one category).
    #[arg(long, value_enum)]
    pub category: IssueCategory,

    /// Ignore the issue. Fails if it is already fully ignored — unignore
    /// first to change its notes or reason.
    #[arg(long)]
    #[serde(default)]
    pub ignore: bool,

    /// Revert a previous ignore, returning the issue to active.
    #[arg(long)]
    #[serde(default)]
    pub unignore: bool,

    /// Free-text comment recorded with --ignore.
    #[arg(long)]
    pub notes: Option<String>,

    /// Structured reason recorded with --ignore.
    #[arg(long, value_enum)]
    pub reason: Option<IssueIgnoreReason>,
}

/// The `update` operation, declared once for both the CLI and the MCP server.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(tag = "entity", rename_all = "snake_case")]
pub enum UpdateCommand {
    /// Update a project.
    #[command(alias = "projects")]
    Project(UpdateProjectParams),
    /// Ignore or unignore an issue.
    #[command(alias = "issues")]
    Issue(UpdateIssueParams),
}

/// The result of an [`UpdateCommand`], serialized as the inner entity.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum UpdateOutput {
    /// The updated project.
    Project(Box<Project>),
    /// The updated issue, refreshed after the write.
    Issue(Box<Issue>),
}

impl PrettyPrint for UpdateOutput {
    fn pretty_print(&self) -> String {
        match self {
            UpdateOutput::Project(p) => p.pretty_print(),
            UpdateOutput::Issue(i) => i.pretty_print(),
        }
    }
}

/// Execute an `update` operation.
pub async fn run_update(client: &FossaClient, command: UpdateCommand) -> Result<UpdateOutput> {
    Ok(match command {
        UpdateCommand::Project(p) => {
            if p.title.is_none()
                && p.description.is_none()
                && p.url.is_none()
                && p.public.is_none()
                && p.policy_id.is_none()
                && p.default_branch.is_none()
            {
                return Err(FossaError::InvalidParams(
                    "update project requires at least one field to change \
                     (title, description, url, public, policy_id, default_branch)"
                        .to_string(),
                ));
            }
            let params = ProjectUpdateParams {
                title: p.title,
                description: p.description,
                url: p.url,
                public: p.public,
                policy_id: p.policy_id,
                default_branch: p.default_branch,
            };
            UpdateOutput::Project(Box::new(Project::update(client, p.locator, params).await?))
        }
        UpdateCommand::Issue(p) => {
            // clap's ArgGroup enforces exactly-one on the CLI; MCP arguments
            // bypass clap, so re-check here.
            let action = match (p.ignore, p.unignore) {
                (true, false) => IssueAction::Ignore {
                    notes: p.notes,
                    reason: p.reason,
                },
                (false, true) => {
                    if p.notes.is_some() || p.reason.is_some() {
                        return Err(FossaError::InvalidParams(
                            "notes and reason only apply when ignoring; unignore removes \
                             the existing resolution"
                                .to_string(),
                        ));
                    }
                    IssueAction::Unignore
                }
                _ => {
                    return Err(FossaError::InvalidParams(
                        "update issue requires exactly one of ignore or unignore".to_string(),
                    ))
                }
            };
            let params = IssueUpdateParams {
                category: p.category,
                action,
            };
            UpdateOutput::Issue(Box::new(Issue::update(client, p.id, params).await?))
        }
    })
}
