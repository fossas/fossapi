//! The `update` verb: modify an entity. Currently only projects are updatable.

use clap::{Args, Subcommand};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{FossaClient, FossaError, PrettyPrint, Project, ProjectUpdateParams, Result, Update};

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

/// The `update` operation, declared once for both the CLI and the MCP server.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(tag = "entity", rename_all = "snake_case")]
pub enum UpdateCommand {
    /// Update a project.
    #[command(alias = "projects")]
    Project(UpdateProjectParams),
}

/// The result of an [`UpdateCommand`], serialized as the inner entity.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum UpdateOutput {
    /// The updated project.
    Project(Project),
}

impl PrettyPrint for UpdateOutput {
    fn pretty_print(&self) -> String {
        match self {
            UpdateOutput::Project(p) => p.pretty_print(),
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
            UpdateOutput::Project(Project::update(client, p.locator, params).await?)
        }
    })
}
