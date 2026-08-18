//! CLI argument parsing tests.
//!
//! These tests define the expected CLI interface. The subcommands parse into
//! the shared operation enums in `fossapi::ops`, which the MCP server also
//! consumes.

use clap::Parser;
use fossapi::cli::{
    Cli, Command, GetCommand, IgnoreCommand, ListCommand, UnignoreCommand, UpdateCommand,
};
use fossapi::ops::{
    GetIssueParams, GetProjectParams, GetRevisionParams, IgnoreIssueParams, ListDependenciesParams,
    ListIssuesParams, ListProjectsParams, ListRevisionsParams, PageArgs, UnignoreIssueParams,
    UpdateProjectParams,
};
use fossapi::{IssueCategory, IssueIgnoreReason};

#[test]
fn test_cli_parses_get_subcommand() {
    let cli = Cli::parse_from(["fossapi", "get", "project", "custom+acme/myapp"]);

    assert!(!cli.json);
    match cli.command {
        Command::Get {
            command: GetCommand::Project(GetProjectParams { locator }),
        } => {
            assert_eq!(locator, "custom+acme/myapp");
        }
        _ => panic!("Expected Get command with Project variant"),
    }
}

#[test]
fn test_cli_parses_list_subcommand() {
    let cli = Cli::parse_from(["fossapi", "list", "projects"]);

    assert!(!cli.json);
    match cli.command {
        Command::List {
            command: ListCommand::Projects(..),
        } => {}
        _ => panic!("Expected List command with Projects variant"),
    }
}

#[test]
fn test_cli_parses_update_subcommand() {
    let cli = Cli::parse_from([
        "fossapi",
        "update",
        "project",
        "custom+acme/myapp",
        "--title",
        "New Title",
    ]);

    assert!(!cli.json);
    match cli.command {
        Command::Update {
            command: UpdateCommand::Project(UpdateProjectParams { locator, title, .. }),
        } => {
            assert_eq!(locator, "custom+acme/myapp");
            assert_eq!(title, Some("New Title".to_string()));
        }
        _ => panic!("Expected Update command"),
    }
}

#[test]
fn test_global_json_flag() {
    // --json before subcommand
    let cli = Cli::parse_from(["fossapi", "--json", "list", "projects"]);
    assert!(cli.json);

    // --json after subcommand args (global flag)
    let cli = Cli::parse_from(["fossapi", "list", "projects", "--json"]);
    assert!(cli.json);
}

#[test]
fn test_list_pagination_args() {
    let cli = Cli::parse_from([
        "fossapi", "list", "projects", "--page", "2", "--count", "50",
    ]);

    match cli.command {
        Command::List {
            command:
                ListCommand::Projects(ListProjectsParams {
                    pagination: PageArgs { page, count },
                }),
        } => {
            assert_eq!(page, Some(2));
            assert_eq!(count, Some(50));
        }
        _ => panic!("Expected List command with Projects variant"),
    }
}

#[test]
fn test_entity_variants() {
    // Project (get uses GetCommand)
    let cli = Cli::parse_from(["fossapi", "get", "project", "loc"]);
    assert!(matches!(
        cli.command,
        Command::Get {
            command: GetCommand::Project(..)
        }
    ));

    // Revision (get uses GetCommand)
    let cli = Cli::parse_from(["fossapi", "get", "revision", "loc"]);
    assert!(matches!(
        cli.command,
        Command::Get {
            command: GetCommand::Revision(..)
        }
    ));

    // Issue (get uses GetCommand with u64 id)
    let cli = Cli::parse_from(["fossapi", "get", "issue", "123"]);
    assert!(matches!(
        cli.command,
        Command::Get {
            command: GetCommand::Issue(GetIssueParams { id: 123, .. })
        }
    ));

    // Dependencies (list uses ListCommand with required revision)
    let cli = Cli::parse_from(["fossapi", "list", "dependencies", "loc"]);
    assert!(matches!(
        cli.command,
        Command::List {
            command: ListCommand::Dependencies(..)
        }
    ));
}

// =============================================================================
// GetCommand type-safe parsing
// =============================================================================

#[test]
fn test_get_project_parses_locator() {
    let cli = Cli::parse_from(["fossapi", "get", "project", "custom+acme/myapp"]);
    match cli.command {
        Command::Get {
            command: GetCommand::Project(GetProjectParams { locator }),
        } => {
            assert_eq!(locator, "custom+acme/myapp");
        }
        _ => panic!("Expected GetCommand::Project"),
    }
}

#[test]
fn test_get_revision_parses_locator() {
    let cli = Cli::parse_from(["fossapi", "get", "revision", "custom+acme/myapp$abc123"]);
    match cli.command {
        Command::Get {
            command: GetCommand::Revision(GetRevisionParams { locator }),
        } => {
            assert_eq!(locator, "custom+acme/myapp$abc123");
        }
        _ => panic!("Expected GetCommand::Revision"),
    }
}

#[test]
fn test_get_issue_parses_numeric_id() {
    let cli = Cli::parse_from(["fossapi", "get", "issue", "12345"]);
    match cli.command {
        Command::Get {
            command: GetCommand::Issue(GetIssueParams { id, category }),
        } => {
            assert_eq!(id, 12345u64);
            assert_eq!(category, None);
        }
        _ => panic!("Expected GetCommand::Issue"),
    }
}

#[test]
fn test_get_issue_with_category() {
    let cli = Cli::parse_from([
        "fossapi",
        "get",
        "issue",
        "12345",
        "--category",
        "licensing",
    ]);
    match cli.command {
        Command::Get {
            command: GetCommand::Issue(GetIssueParams { id, category }),
        } => {
            assert_eq!(id, 12345u64);
            assert_eq!(category, Some(IssueCategory::Licensing));
        }
        _ => panic!("Expected GetCommand::Issue"),
    }
}

#[test]
fn test_get_issue_rejects_unknown_category() {
    let result = Cli::try_parse_from(["fossapi", "get", "issue", "12345", "--category", "bogus"]);
    assert!(result.is_err(), "Expected unknown category to be rejected");
}

// =============================================================================
// ListCommand type-safe parsing
// =============================================================================

#[test]
fn test_list_projects_parses() {
    let cli = Cli::parse_from(["fossapi", "list", "projects"]);
    match cli.command {
        Command::List {
            command: ListCommand::Projects(ListProjectsParams { pagination }),
        } => {
            assert_eq!(pagination.page, None);
            assert_eq!(pagination.count, None);
        }
        _ => panic!("Expected ListCommand::Projects"),
    }
}

#[test]
fn test_list_projects_with_pagination() {
    let cli = Cli::parse_from([
        "fossapi", "list", "projects", "--page", "2", "--count", "50",
    ]);
    match cli.command {
        Command::List {
            command: ListCommand::Projects(ListProjectsParams { pagination }),
        } => {
            assert_eq!(pagination.page, Some(2));
            assert_eq!(pagination.count, Some(50));
        }
        _ => panic!("Expected ListCommand::Projects"),
    }
}

#[test]
fn test_list_issues_parses() {
    let cli = Cli::parse_from(["fossapi", "list", "issues", "--category", "vulnerability"]);
    match cli.command {
        Command::List {
            command:
                ListCommand::Issues(ListIssuesParams {
                    category,
                    pagination,
                }),
        } => {
            assert_eq!(pagination.page, None);
            assert_eq!(pagination.count, None);
            assert_eq!(category, IssueCategory::Vulnerability);
        }
        _ => panic!("Expected ListCommand::Issues"),
    }
}

#[test]
fn test_list_issues_requires_category() {
    let result = Cli::try_parse_from(["fossapi", "list", "issues"]);
    assert!(result.is_err(), "Expected --category to be required");
}

#[test]
fn test_list_dependencies_requires_revision_arg() {
    let cli = Cli::parse_from(["fossapi", "list", "dependencies", "custom+org/repo$abc"]);
    match cli.command {
        Command::List {
            command: ListCommand::Dependencies(ListDependenciesParams { revision, .. }),
        } => {
            assert_eq!(revision, "custom+org/repo$abc");
        }
        _ => panic!("Expected ListCommand::Dependencies"),
    }
}

#[test]
fn test_list_dependencies_without_revision_is_rejected() {
    let result = Cli::try_parse_from(["fossapi", "list", "dependencies"]);
    assert!(result.is_err(), "Expected revision to be required");
}

#[test]
fn test_list_revisions_requires_project_arg() {
    let cli = Cli::parse_from(["fossapi", "list", "revisions", "custom+org/repo"]);
    match cli.command {
        Command::List {
            command: ListCommand::Revisions(ListRevisionsParams { project, .. }),
        } => {
            assert_eq!(project, "custom+org/repo");
        }
        _ => panic!("Expected ListCommand::Revisions"),
    }
}

#[test]
fn test_list_issues_with_pagination() {
    let cli = Cli::parse_from([
        "fossapi",
        "list",
        "issues",
        "--page",
        "3",
        "--count",
        "25",
        "--category",
        "quality",
    ]);
    match cli.command {
        Command::List {
            command:
                ListCommand::Issues(ListIssuesParams {
                    category,
                    pagination,
                }),
        } => {
            assert_eq!(pagination.page, Some(3));
            assert_eq!(pagination.count, Some(25));
            assert_eq!(category, IssueCategory::Quality);
        }
        _ => panic!("Expected ListCommand::Issues"),
    }
}

#[test]
fn test_list_revisions_with_pagination() {
    let cli = Cli::parse_from([
        "fossapi",
        "list",
        "revisions",
        "custom+org/repo",
        "--page",
        "2",
    ]);
    match cli.command {
        Command::List {
            command:
                ListCommand::Revisions(ListRevisionsParams {
                    project,
                    pagination,
                }),
        } => {
            assert_eq!(project, "custom+org/repo");
            assert_eq!(pagination.page, Some(2));
            assert_eq!(pagination.count, None);
        }
        _ => panic!("Expected ListCommand::Revisions"),
    }
}

// =============================================================================
// UpdateCommand CLI parsing
// =============================================================================

#[test]
fn test_update_project_parses_locator() {
    let cli = Cli::parse_from(["fossapi", "update", "project", "custom+acme/myapp"]);
    match cli.command {
        Command::Update {
            command: UpdateCommand::Project(UpdateProjectParams { locator, .. }),
        } => {
            assert_eq!(locator, "custom+acme/myapp");
        }
        _ => panic!("Expected Update command"),
    }
}

#[test]
fn test_update_project_title_flag() {
    let cli = Cli::parse_from([
        "fossapi",
        "update",
        "project",
        "custom+acme/myapp",
        "--title",
        "New Title",
    ]);
    match cli.command {
        Command::Update {
            command: UpdateCommand::Project(UpdateProjectParams { title, .. }),
        } => {
            assert_eq!(title, Some("New Title".to_string()));
        }
        _ => panic!("Expected Update command"),
    }
}

#[test]
fn test_update_project_public_flag() {
    let cli = Cli::parse_from([
        "fossapi",
        "update",
        "project",
        "custom+acme/myapp",
        "--public",
        "true",
    ]);
    match cli.command {
        Command::Update {
            command: UpdateCommand::Project(UpdateProjectParams { public, .. }),
        } => {
            assert_eq!(public, Some(true));
        }
        _ => panic!("Expected Update command"),
    }
}

#[test]
fn test_update_project_url_flag() {
    let cli = Cli::parse_from([
        "fossapi",
        "update",
        "project",
        "custom+acme/myapp",
        "--url",
        "https://example.com/repo",
    ]);
    match cli.command {
        Command::Update {
            command: UpdateCommand::Project(UpdateProjectParams { url, .. }),
        } => {
            assert_eq!(url, Some("https://example.com/repo".to_string()));
        }
        _ => panic!("Expected Update command"),
    }
}

#[test]
fn test_update_project_multiple_flags() {
    let cli = Cli::parse_from([
        "fossapi",
        "update",
        "project",
        "custom+acme/myapp",
        "--title",
        "New Title",
        "--public",
        "false",
    ]);
    match cli.command {
        Command::Update {
            command:
                UpdateCommand::Project(UpdateProjectParams {
                    locator,
                    title,
                    public,
                    ..
                }),
        } => {
            assert_eq!(locator, "custom+acme/myapp");
            assert_eq!(title, Some("New Title".to_string()));
            assert_eq!(public, Some(false));
        }
        _ => panic!("Expected Update command"),
    }
}

// =============================================================================
// MCP CLI subcommand
// =============================================================================

#[test]
fn test_cli_parses_mcp_subcommand() {
    let cli = Cli::parse_from(["fossapi", "mcp"]);
    match cli.command {
        Command::Mcp { verbose } => {
            assert!(!verbose);
        }
        _ => panic!("Expected Mcp command"),
    }
}

#[test]
fn test_cli_parses_mcp_with_verbose_flag() {
    let cli = Cli::parse_from(["fossapi", "mcp", "--verbose"]);
    match cli.command {
        Command::Mcp { verbose } => {
            assert!(verbose);
        }
        _ => panic!("Expected Mcp command"),
    }
}

// =============================================================================
// ignore / unignore verbs
// =============================================================================

#[test]
fn test_ignore_issue_with_notes_and_reason() {
    let cli = Cli::parse_from([
        "fossapi",
        "ignore",
        "issue",
        "987654",
        "--category",
        "vulnerability",
        "--notes",
        "false positive patch",
        "--reason",
        "other",
    ]);
    match cli.command {
        Command::Ignore {
            command:
                IgnoreCommand::Issue(IgnoreIssueParams {
                    id,
                    category,
                    notes,
                    reason,
                }),
        } => {
            assert_eq!(id, 987654);
            assert_eq!(category, IssueCategory::Vulnerability);
            assert_eq!(notes, Some("false positive patch".to_string()));
            assert_eq!(reason, Some(IssueIgnoreReason::Other));
        }
        _ => panic!("Expected Ignore command"),
    }
}

// Multi-word reasons are where the CLI and API surfaces diverge: clap derives
// kebab-case value names, while the API stores `Vulnerable_code_not_in_execute_path`.
#[test]
fn test_ignore_issue_reason_uses_kebab_case_value_names() {
    let cli = Cli::parse_from([
        "fossapi",
        "ignore",
        "issue",
        "987654",
        "--category",
        "vulnerability",
        "--reason",
        "vulnerable-code-not-in-execute-path",
    ]);
    match cli.command {
        Command::Ignore {
            command: IgnoreCommand::Issue(IgnoreIssueParams { reason, .. }),
        } => {
            assert_eq!(
                reason,
                Some(IssueIgnoreReason::VulnerableCodeNotInExecutePath)
            );
        }
        _ => panic!("Expected Ignore command"),
    }
}

#[test]
fn test_unignore_issue() {
    let cli = Cli::parse_from([
        "fossapi",
        "unignore",
        "issue",
        "987654",
        "--category",
        "licensing",
    ]);
    match cli.command {
        Command::Unignore {
            command: UnignoreCommand::Issue(UnignoreIssueParams { id, category }),
        } => {
            assert_eq!(id, 987654);
            assert_eq!(category, IssueCategory::Licensing);
        }
        _ => panic!("Expected Unignore command"),
    }
}

#[test]
fn test_ignore_issue_requires_category() {
    let result = Cli::try_parse_from(["fossapi", "ignore", "issue", "987654"]);
    assert!(result.is_err(), "--category must be required");
}

#[test]
fn test_unignore_issue_requires_category() {
    let result = Cli::try_parse_from(["fossapi", "unignore", "issue", "987654"]);
    assert!(result.is_err(), "--category must be required");
}

#[test]
fn test_unignore_issue_has_no_notes_flag() {
    // notes belong to ignore only; the unignore declaration has no such
    // field, so clap rejects the flag outright.
    let result = Cli::try_parse_from([
        "fossapi",
        "unignore",
        "issue",
        "987654",
        "--category",
        "licensing",
        "--notes",
        "orphan comment",
    ]);
    assert!(result.is_err(), "--notes is not a flag of unignore");
}
