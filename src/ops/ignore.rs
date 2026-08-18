//! The `ignore` and `unignore` verbs: issue status transitions.
//!
//! Ignore/unignore are first-class verbs (not `update` flags) because they
//! are the domain's own names for the actions, and separate MCP tools let
//! agent harnesses permission-gate writes independently of metadata updates.

use clap::{Args, Subcommand};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    FossaClient, Issue, IssueAction, IssueCategory, IssueIgnoreReason, IssueUpdateParams,
    PrettyPrint, Result, Update,
};

/// Parameters for `ignore issue`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct IgnoreIssueParams {
    /// The issue ID.
    pub id: u64,

    /// Issue category (required for writes; the API scopes them to one category).
    #[arg(long, value_enum)]
    pub category: IssueCategory,

    /// Free-text comment recorded with the ignore.
    #[arg(long)]
    pub notes: Option<String>,

    /// Structured reason recorded with the ignore (vulnerability issues only).
    #[arg(long, value_enum)]
    pub reason: Option<IssueIgnoreReason>,
}

/// Parameters for `unignore issue`.
#[derive(Args, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct UnignoreIssueParams {
    /// The issue ID.
    pub id: u64,

    /// Issue category (required for writes; the API scopes them to one category).
    #[arg(long, value_enum)]
    pub category: IssueCategory,
}

/// The `ignore` operation, declared once for both the CLI and the MCP server.
///
/// Ignoring an issue that is already fully ignored is refused with a prompt
/// to unignore first (see ADR 0002); requires a full API token.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(tag = "entity", rename_all = "snake_case")]
pub enum IgnoreCommand {
    /// Ignore an issue, optionally with notes and a reason.
    #[command(alias = "issues")]
    Issue(IgnoreIssueParams),
}

/// The `unignore` operation, declared once for both the CLI and the MCP server.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(tag = "entity", rename_all = "snake_case")]
pub enum UnignoreCommand {
    /// Revert a previous ignore, returning the issue to active.
    #[command(alias = "issues")]
    Issue(UnignoreIssueParams),
}

/// The result of an [`IgnoreCommand`], serialized as the inner entity.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum IgnoreOutput {
    /// The ignored issue, refreshed after the write.
    Issue(Box<Issue>),
}

/// The result of an [`UnignoreCommand`], serialized as the inner entity.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum UnignoreOutput {
    /// The unignored issue, refreshed after the write.
    Issue(Box<Issue>),
}

impl PrettyPrint for IgnoreOutput {
    fn pretty_print(&self) -> String {
        match self {
            IgnoreOutput::Issue(i) => i.pretty_print(),
        }
    }
}

impl PrettyPrint for UnignoreOutput {
    fn pretty_print(&self) -> String {
        match self {
            UnignoreOutput::Issue(i) => i.pretty_print(),
        }
    }
}

/// Execute an `ignore` operation.
pub async fn run_ignore(client: &FossaClient, command: IgnoreCommand) -> Result<IgnoreOutput> {
    Ok(match command {
        IgnoreCommand::Issue(p) => {
            let params = IssueUpdateParams {
                category: p.category,
                action: IssueAction::Ignore {
                    notes: p.notes,
                    reason: p.reason,
                },
            };
            IgnoreOutput::Issue(Box::new(Issue::update(client, p.id, params).await?))
        }
    })
}

/// Execute an `unignore` operation.
pub async fn run_unignore(
    client: &FossaClient,
    command: UnignoreCommand,
) -> Result<UnignoreOutput> {
    Ok(match command {
        UnignoreCommand::Issue(p) => {
            let params = IssueUpdateParams {
                category: p.category,
                action: IssueAction::Unignore,
            };
            UnignoreOutput::Issue(Box::new(Issue::update(client, p.id, params).await?))
        }
    })
}
