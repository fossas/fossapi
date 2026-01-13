//! CLI argument parsing tests (TDD RED phase)
//!
//! These tests define the expected CLI interface. Written BEFORE implementation.

use clap::Parser;
use fossapi::cli::{Cli, Command, Entity};

#[test]
fn test_cli_parses_get_subcommand() {
    let cli = Cli::parse_from(["fossapi", "get", "project", "custom+acme/myapp"]);

    assert!(!cli.json);
    match cli.command {
        Command::Get { entity, locator } => {
            assert!(matches!(entity, Entity::Project));
            assert_eq!(locator, "custom+acme/myapp");
        }
        _ => panic!("Expected Get command"),
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
    // Project
    let cli = Cli::parse_from(["fossapi", "get", "project", "loc"]);
    assert!(matches!(cli.command, Command::Get { entity: Entity::Project, .. }));

    // Revision
    let cli = Cli::parse_from(["fossapi", "get", "revision", "loc"]);
    assert!(matches!(cli.command, Command::Get { entity: Entity::Revision, .. }));

    // Issue
    let cli = Cli::parse_from(["fossapi", "get", "issue", "123"]);
    assert!(matches!(cli.command, Command::Get { entity: Entity::Issue, .. }));

    // Dependencies (list only)
    let cli = Cli::parse_from(["fossapi", "list", "dependencies", "--revision", "loc"]);
    assert!(matches!(cli.command, Command::List { entity: Entity::Dependency, .. }));
}
