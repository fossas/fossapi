//! CLI argument parsing tests (TDD RED phase)
//!
//! These tests define the expected CLI interface. Written BEFORE implementation.

use clap::Parser;
use fossapi::cli::{Cli, Command, Entity, GetCommand, ListCommand};
use fossapi::IssueCategory;

#[test]
fn test_cli_parses_get_subcommand() {
    let cli = Cli::parse_from(["fossapi", "get", "project", "custom+acme/myapp"]);

    assert!(!cli.json);
    match cli.command {
        Command::Get { command: GetCommand::Project { locator } } => {
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
        Command::List { command: ListCommand::Projects { .. } } => {}
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
            entity,
            locator,
            args,
        } => {
            assert!(matches!(entity, Entity::Project));
            assert_eq!(locator, "custom+acme/myapp");
            assert_eq!(args.title, Some("New Title".to_string()));
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
    let cli = Cli::parse_from(["fossapi", "list", "projects", "--page", "2", "--count", "50"]);

    match cli.command {
        Command::List { command: ListCommand::Projects { page, count } } => {
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
    assert!(matches!(cli.command, Command::Get { command: GetCommand::Project { .. } }));

    // Revision (get uses GetCommand)
    let cli = Cli::parse_from(["fossapi", "get", "revision", "loc"]);
    assert!(matches!(cli.command, Command::Get { command: GetCommand::Revision { .. } }));

    // Issue (get uses GetCommand with u64 id)
    let cli = Cli::parse_from(["fossapi", "get", "issue", "123"]);
    assert!(matches!(cli.command, Command::Get { command: GetCommand::Issue { id: 123, .. } }));

    // Dependencies (list uses ListCommand with required revision)
    let cli = Cli::parse_from(["fossapi", "list", "dependencies", "loc"]);
    assert!(matches!(cli.command, Command::List { command: ListCommand::Dependencies { .. } }));
}

// =============================================================================
// TDD Tests for ISS-10843: GetCommand type-safe parsing
// =============================================================================

#[test]
fn test_get_project_parses_locator() {
    let cli = Cli::parse_from(["fossapi", "get", "project", "custom+acme/myapp"]);
    match cli.command {
        Command::Get { command: GetCommand::Project { locator } } => {
            assert_eq!(locator, "custom+acme/myapp");
        }
        _ => panic!("Expected GetCommand::Project"),
    }
}

#[test]
fn test_get_revision_parses_locator() {
    let cli = Cli::parse_from(["fossapi", "get", "revision", "custom+acme/myapp$abc123"]);
    match cli.command {
        Command::Get { command: GetCommand::Revision { locator } } => {
            assert_eq!(locator, "custom+acme/myapp$abc123");
        }
        _ => panic!("Expected GetCommand::Revision"),
    }
}

#[test]
fn test_get_issue_parses_numeric_id() {
    let cli = Cli::parse_from(["fossapi", "get", "issue", "12345"]);
    match cli.command {
        Command::Get { command: GetCommand::Issue { id, category } } => {
            assert_eq!(id, 12345u64);
            assert_eq!(category, None);
        }
        _ => panic!("Expected GetCommand::Issue"),
    }
}

#[test]
fn test_get_issue_with_category() {
    let cli = Cli::parse_from(["fossapi", "get", "issue", "12345", "--category", "licensing"]);
    match cli.command {
        Command::Get { command: GetCommand::Issue { id, category } } => {
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
// TDD Tests for ISS-10844: ListCommand type-safe parsing
// =============================================================================

#[test]
fn test_list_projects_parses() {
    let cli = Cli::parse_from(["fossapi", "list", "projects"]);
    match cli.command {
        Command::List { command: ListCommand::Projects { page, count } } => {
            assert_eq!(page, None);
            assert_eq!(count, None);
        }
        _ => panic!("Expected ListCommand::Projects"),
    }
}

#[test]
fn test_list_projects_with_pagination() {
    let cli = Cli::parse_from(["fossapi", "list", "projects", "--page", "2", "--count", "50"]);
    match cli.command {
        Command::List { command: ListCommand::Projects { page, count } } => {
            assert_eq!(page, Some(2));
            assert_eq!(count, Some(50));
        }
        _ => panic!("Expected ListCommand::Projects"),
    }
}

#[test]
fn test_list_issues_parses() {
    let cli = Cli::parse_from(["fossapi", "list", "issues", "--category", "vulnerability"]);
    match cli.command {
        Command::List { command: ListCommand::Issues { page, count, category } } => {
            assert_eq!(page, None);
            assert_eq!(count, None);
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
        Command::List { command: ListCommand::Dependencies { revision, revision_positional } } => {
            assert_eq!(revision, None);
            assert_eq!(revision_positional, Some("custom+org/repo$abc".to_string()));
        }
        _ => panic!("Expected ListCommand::Dependencies"),
    }
}

#[test]
fn test_list_revisions_requires_project_arg() {
    let cli = Cli::parse_from(["fossapi", "list", "revisions", "custom+org/repo"]);
    match cli.command {
        Command::List { command: ListCommand::Revisions { project, .. } } => {
            assert_eq!(project, "custom+org/repo");
        }
        _ => panic!("Expected ListCommand::Revisions"),
    }
}

#[test]
fn test_list_issues_with_pagination() {
    let cli = Cli::parse_from([
        "fossapi", "list", "issues", "--page", "3", "--count", "25", "--category", "quality",
    ]);
    match cli.command {
        Command::List { command: ListCommand::Issues { page, count, category } } => {
            assert_eq!(page, Some(3));
            assert_eq!(count, Some(25));
            assert_eq!(category, IssueCategory::Quality);
        }
        _ => panic!("Expected ListCommand::Issues"),
    }
}

#[test]
fn test_list_revisions_with_pagination() {
    let cli = Cli::parse_from(["fossapi", "list", "revisions", "custom+org/repo", "--page", "2"]);
    match cli.command {
        Command::List { command: ListCommand::Revisions { project, page, count } } => {
            assert_eq!(project, "custom+org/repo");
            assert_eq!(page, Some(2));
            assert_eq!(count, None);
        }
        _ => panic!("Expected ListCommand::Revisions"),
    }
}

// =============================================================================
// TDD Tests for ISS-10849: --revision flag for list dependencies
// =============================================================================

#[test]
fn test_list_dependencies_with_revision_flag() {
    let cli = Cli::parse_from(["fossapi", "list", "dependencies", "--revision", "custom+org/repo$abc"]);
    match cli.command {
        Command::List { command: ListCommand::Dependencies { revision, revision_positional } } => {
            assert_eq!(revision, Some("custom+org/repo$abc".to_string()));
            assert_eq!(revision_positional, None);
        }
        _ => panic!("Expected ListCommand::Dependencies"),
    }
}

// =============================================================================
// TDD Tests for ISS-10845: UpdateCommand CLI parsing
// =============================================================================

#[test]
fn test_update_project_parses_locator() {
    let cli = Cli::parse_from(["fossapi", "update", "project", "custom+acme/myapp"]);
    match cli.command {
        Command::Update { entity, locator, .. } => {
            assert!(matches!(entity, Entity::Project));
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
        Command::Update { args, .. } => {
            assert_eq!(args.title, Some("New Title".to_string()));
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
        Command::Update { args, .. } => {
            assert_eq!(args.public, Some(true));
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
            entity,
            locator,
            args,
        } => {
            assert!(matches!(entity, Entity::Project));
            assert_eq!(locator, "custom+acme/myapp");
            assert_eq!(args.title, Some("New Title".to_string()));
            assert_eq!(args.public, Some(false));
        }
        _ => panic!("Expected Update command"),
    }
}

// =============================================================================
// TDD Tests for ISS-10860: MCP CLI subcommand
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
// Issue update flags (ignore / unignore with comment)
// =============================================================================

#[test]
fn test_update_issue_ignore_with_notes_and_reason() {
    let cli = Cli::parse_from([
        "fossapi",
        "update",
        "issue",
        "987654",
        "--category",
        "licensing",
        "--ignore",
        "--notes",
        "false positive patch",
        "--reason",
        "other",
    ]);
    match cli.command {
        Command::Update {
            entity,
            locator,
            args,
        } => {
            assert!(matches!(entity, Entity::Issue));
            assert_eq!(locator, "987654");
            assert_eq!(args.category, Some(IssueCategory::Licensing));
            assert!(args.ignore);
            assert!(!args.unignore);
            assert_eq!(args.notes, Some("false positive patch".to_string()));
            assert_eq!(args.reason, Some(fossapi::IssueIgnoreReason::Other));
        }
        _ => panic!("Expected Update command"),
    }
}

#[test]
fn test_update_issue_unignore() {
    let cli = Cli::parse_from([
        "fossapi",
        "update",
        "issue",
        "987654",
        "--category",
        "vulnerability",
        "--unignore",
    ]);
    match cli.command {
        Command::Update { args, .. } => {
            assert!(args.unignore);
            assert!(!args.ignore);
        }
        _ => panic!("Expected Update command"),
    }
}

#[test]
fn test_update_issue_ignore_conflicts_with_unignore() {
    let result = Cli::try_parse_from([
        "fossapi", "update", "issue", "987654", "--ignore", "--unignore",
    ]);
    assert!(result.is_err());
}

#[test]
fn test_update_issue_notes_requires_ignore() {
    let result = Cli::try_parse_from([
        "fossapi", "update", "issue", "987654", "--notes", "orphan comment",
    ]);
    assert!(result.is_err());
}
