//! TDD tests for CLI output formatting (ISS-10846)
//!
//! RED phase: These tests define the expected behavior for:
//! - JSON output with --json flag
//! - Pretty-print output as default

use fossapi::{Issue, Project, PrettyPrint, Revision};

// ============================================================================
// JSON Output Tests
// ============================================================================

#[test]
fn test_json_flag_outputs_valid_json() {
    // When --json is used, output must be valid parseable JSON
    let project = make_test_project();
    let json_output = serde_json::to_string_pretty(&project).unwrap();

    // Verify it's valid JSON by parsing it back
    let parsed: serde_json::Value = serde_json::from_str(&json_output).unwrap();
    assert!(parsed.is_object());
    assert_eq!(parsed["title"], "Test Project");
}

#[test]
fn test_json_flag_for_list_outputs_array() {
    // When --json is used with list commands, output must be a JSON array
    let projects = vec![make_test_project(), make_test_project()];
    let json_output = serde_json::to_string_pretty(&projects).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json_output).unwrap();
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 2);
}

#[test]
fn test_json_output_preserves_all_fields() {
    // JSON output must preserve all model fields
    let project = make_test_project();
    let json_output = serde_json::to_string_pretty(&project).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_output).unwrap();

    // Core fields must be present
    assert!(parsed.get("id").is_some());
    assert!(parsed.get("title").is_some());
    assert!(parsed.get("public").is_some());
}

// ============================================================================
// Pretty-Print Tests
// ============================================================================

#[test]
fn test_default_output_is_not_json() {
    // Default output (no --json) must NOT be JSON
    let project = make_test_project();
    let pretty_output = project.pretty_print();

    // Attempting to parse as JSON should fail
    let parse_result: Result<serde_json::Value, _> = serde_json::from_str(&pretty_output);
    assert!(
        parse_result.is_err(),
        "Default output should NOT be valid JSON"
    );
}

#[test]
fn test_project_pretty_print_shows_key_fields() {
    // Project pretty-print must show: Locator, Title, Issues
    let project = make_test_project();
    let output = project.pretty_print();

    assert!(
        output.contains("custom+123/test-project"),
        "Should show locator"
    );
    assert!(output.contains("Test Project"), "Should show title");
    assert!(output.contains("Title"), "Should have Title label");
}

#[test]
fn test_list_pretty_print_is_table() {
    // List output should be tabular (this tests the existing table behavior)
    // The table output from `tabled` contains column separators
    use tabled::{Table, Tabled};

    #[derive(Tabled)]
    struct TestRow {
        name: String,
        value: String,
    }

    let rows = vec![
        TestRow {
            name: "a".to_string(),
            value: "1".to_string(),
        },
        TestRow {
            name: "b".to_string(),
            value: "2".to_string(),
        },
    ];

    let table_output = Table::new(rows).to_string();

    // Table output has horizontal lines and column alignment
    assert!(table_output.contains("name"), "Should have column headers");
    assert!(table_output.contains("value"), "Should have column headers");
}

#[test]
fn test_issue_pretty_print_shows_severity() {
    // Issue pretty-print must show severity field
    let issue = make_test_issue();
    let output = issue.pretty_print();

    assert!(output.contains("high"), "Should show severity");
    assert!(output.contains("Severity"), "Should have Severity label");
    assert!(output.contains("vulnerability"), "Should show issue type");
}

#[test]
fn test_revision_pretty_print_shows_key_fields() {
    // Revision pretty-print must show: Locator, Resolved, Source
    let revision = make_test_revision();
    let output = revision.pretty_print();

    assert!(
        output.contains("custom+org/project$main"),
        "Should show locator"
    );
    assert!(
        output.contains("Resolved") || output.contains("resolved"),
        "Should show resolved status"
    );
}

// ============================================================================
// Test Helpers
// ============================================================================

fn make_test_project() -> Project {
    serde_json::from_value(serde_json::json!({
        "id": "custom+123/test-project",
        "title": "Test Project",
        "public": false,
        "labels": [],
        "teams": [],
        "issues": {
            "total": 5,
            "licensing": 2,
            "security": 3,
            "quality": 0
        }
    }))
    .unwrap()
}

fn make_test_issue() -> Issue {
    serde_json::from_value(serde_json::json!({
        "id": 12345,
        "type": "vulnerability",
        "source": { "id": "npm+lodash$4.17.0" },
        "depths": { "direct": 1, "deep": 0 },
        "statuses": { "active": 1, "ignored": 0 },
        "projects": [],
        "severity": "high",
        "cve": "CVE-2021-1234"
    }))
    .unwrap()
}

fn make_test_revision() -> Revision {
    serde_json::from_value(serde_json::json!({
        "locator": "custom+org/project$main",
        "resolved": true,
        "source": "cli",
        "sourceType": "cargo"
    }))
    .unwrap()
}
