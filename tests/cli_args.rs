//! CLI argument parsing tests (TDD RED phase)
//!
//! These tests define the expected CLI interface. Written BEFORE implementation.

use clap::Parser;
use fossapi::cli::{Cli, Command, Entity, GetCommand};

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
        Command::List { entity, .. } => {
            assert!(matches!(entity, Entity::Project));
        }
        _ => panic!("Expected List command"),
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
            title,
            ..
        } => {
            assert!(matches!(entity, Entity::Project));
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

    // --json after subcommand (global flag)
    let cli = Cli::parse_from(["fossapi", "list", "projects", "--json"]);
    assert!(cli.json);
}

#[test]
fn test_list_pagination_args() {
    let cli = Cli::parse_from(["fossapi", "list", "projects", "--page", "2", "--count", "50"]);

    match cli.command {
        Command::List { page, count, .. } => {
            assert_eq!(page, Some(2));
            assert_eq!(count, Some(50));
        }
        _ => panic!("Expected List command"),
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
    assert!(matches!(cli.command, Command::Get { command: GetCommand::Issue { id: 123 } }));

    // Dependencies (list only, still uses Entity)
    let cli = Cli::parse_from(["fossapi", "list", "dependencies", "--revision", "loc"]);
    assert!(matches!(cli.command, Command::List { entity: Entity::Dependency, .. }));
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
        Command::Get { command: GetCommand::Issue { id } } => {
            assert_eq!(id, 12345u64);
        }
        _ => panic!("Expected GetCommand::Issue"),
    }
}
