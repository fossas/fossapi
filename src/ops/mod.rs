//! Shared operation definitions consumed by both the CLI and the MCP server.
//!
//! Each verb (`get`, `list`, `update`) is declared once as an enum whose
//! variants wrap per-entity parameter structs. The same declaration drives
//! both frontends:
//!
//! - **CLI**: the enums derive [`clap::Subcommand`] and the param structs
//!   derive [`clap::Args`], so `fossapi get issue 123 --category licensing`
//!   parses directly into [`GetCommand`].
//! - **MCP**: the enums derive [`serde::Deserialize`] (internally tagged on
//!   `entity`) and [`schemars::JsonSchema`], so `{"entity": "issue", "id": 123}`
//!   deserializes into the same [`GetCommand`] and the tool's input schema is
//!   generated from it.
//!
//! Because both surfaces consume one declaration, adding an entity is: add a
//! param struct, add an enum variant, and add a match arm to the verb's `run`
//! function — the compiler then guarantees both surfaces support it. Parity
//! between the surfaces is checked by `tests/parity.rs`.

mod get;
mod list;
mod update;

pub use get::{
    run_get, GetCommand, GetIssueParams, GetOutput, GetProjectParams, GetRevisionParams,
    GetSnippetMatchParams, GetSnippetParams,
};
pub use list::{
    run_list, ListCommand, ListDependenciesParams, ListIssuesParams, ListOutput,
    ListProjectsParams, ListRevisionsParams, ListSnippetLocationsParams, ListSnippetPathsParams,
    ListSnippetsParams,
};
pub use update::{run_update, UpdateCommand, UpdateOutput, UpdateProjectParams};

use schemars::JsonSchema;
use serde::Deserialize;

/// Pagination arguments shared by paged list operations.
///
/// This struct is flattened into each paged operation for both surfaces
/// (`#[command(flatten)]` for clap, `#[serde(flatten)]` for MCP), and
/// [`PageArgs::resolve`] is the single source of pagination policy.
#[derive(clap::Args, Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct PageArgs {
    /// Page number (1-indexed).
    #[arg(long)]
    pub page: Option<u32>,

    /// Number of items per page (max 100).
    #[arg(long)]
    pub count: Option<u32>,
}

impl PageArgs {
    /// Default page size when `--count` is not given.
    pub const DEFAULT_COUNT: u32 = 20;
    /// Hard cap on page size.
    pub const MAX_COUNT: u32 = 100;

    /// Resolve to a concrete `(page, count)`, applying defaults and the cap.
    pub fn resolve(self) -> (u32, u32) {
        (
            self.page.unwrap_or(1),
            self.count
                .unwrap_or(Self::DEFAULT_COUNT)
                .min(Self::MAX_COUNT),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_args_resolve_defaults() {
        assert_eq!(PageArgs::default().resolve(), (1, 20));
    }

    #[test]
    fn page_args_resolve_caps_count() {
        let args = PageArgs {
            page: Some(3),
            count: Some(500),
        };
        assert_eq!(args.resolve(), (3, 100));
    }
}
