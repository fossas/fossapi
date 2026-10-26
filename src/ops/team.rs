//! The `add`, `remove`, and `move` verbs: team project membership.
//!
//! Separate verbs (not `update` flags) for the same reason as ignore/unignore:
//! they are the domain's own names, and separate MCP tools let agent
//! harnesses permission-gate removals independently of additions.

use clap::{Args, Subcommand};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    add_team_projects, move_team_projects, remove_team_projects, FossaClient, Result, TeamMove,
    TeamProjects,
};

/// Parameters for `add project`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct AddProjectParams {
    /// Project locators (e.g., "custom+org/repo"); any "$revision" suffix is stripped.
    #[arg(required = true)]
    pub locators: Vec<String>,

    /// Numeric ID of the team to add the projects to.
    #[arg(long)]
    pub team: u64,
}

/// Parameters for `remove project`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct RemoveProjectParams {
    /// Project locators (e.g., "custom+org/repo"); any "$revision" suffix is stripped.
    #[arg(required = true)]
    pub locators: Vec<String>,

    /// Numeric ID of the team to remove the projects from.
    #[arg(long)]
    pub team: u64,
}

/// Parameters for `move project`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct MoveProjectParams {
    /// Project locators (e.g., "custom+org/repo"); any "$revision" suffix is stripped.
    #[arg(required = true)]
    pub locators: Vec<String>,

    /// Numeric ID of the team to take the projects off.
    #[arg(long)]
    pub from: u64,

    /// Numeric ID of the team to put the projects on.
    #[arg(long)]
    pub to: u64,
}

/// The `add` operation, declared once for both the CLI and the MCP server.
///
/// Adding keeps the projects' other teams. All-or-nothing: the API refuses
/// the whole batch if the token lacks Edit on any project.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(tag = "entity", rename_all = "snake_case")]
pub enum AddCommand {
    /// Add projects to a team.
    #[command(alias = "projects")]
    Project(AddProjectParams),
}

/// The `remove` operation, declared once for both the CLI and the MCP server.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(tag = "entity", rename_all = "snake_case")]
pub enum RemoveCommand {
    /// Remove projects from a team.
    #[command(alias = "projects")]
    Project(RemoveProjectParams),
}

/// The `move` operation, declared once for both the CLI and the MCP server.
///
/// There is no move endpoint: this adds to the target team, then removes
/// from the source team. If the add fails nothing is removed.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(tag = "entity", rename_all = "snake_case")]
pub enum MoveCommand {
    /// Move projects from one team to another.
    #[command(alias = "projects")]
    Project(MoveProjectParams),
}

/// Execute an `add` operation; returns the team's membership after the write.
pub async fn run_add(client: &FossaClient, command: AddCommand) -> Result<TeamProjects> {
    match command {
        AddCommand::Project(p) => add_team_projects(client, p.team, &p.locators).await,
    }
}

/// Execute a `remove` operation; returns the team's membership after the write.
pub async fn run_remove(client: &FossaClient, command: RemoveCommand) -> Result<TeamProjects> {
    match command {
        RemoveCommand::Project(p) => remove_team_projects(client, p.team, &p.locators).await,
    }
}

/// Execute a `move` operation; returns both teams' membership after the writes.
pub async fn run_move(client: &FossaClient, command: MoveCommand) -> Result<TeamMove> {
    match command {
        MoveCommand::Project(p) => move_team_projects(client, &p.locators, p.from, p.to).await,
    }
}
