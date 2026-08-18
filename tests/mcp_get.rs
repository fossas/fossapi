//! Tests for the MCP get tool.
//!
//! Uses wiremock to mock the FOSSA API. Tool arguments are deserialized into
//! the shared `fossapi::ops::GetCommand`, exactly as `call_tool` does, so
//! these tests also cover the JSON wire format.

use fossapi::mcp::FossaServer;
use fossapi::ops::GetCommand;
use fossapi::FossaClient;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Deserialize tool arguments the same way call_tool does.
fn get_command(args: serde_json::Value) -> serde_json::Result<GetCommand> {
    serde_json::from_value(args)
}

/// Extract text from CallToolResult content.
fn extract_text(result: &rmcp::model::CallToolResult) -> &str {
    let content = &result.content[0];
    content
        .raw
        .as_text()
        .expect("Expected text content")
        .text
        .as_str()
}

#[tokio::test]
async fn test_mcp_get_project_returns_json() {
    let mock_server = MockServer::start().await;

    let project_json = serde_json::json!({
        "id": "custom+123/test-project",
        "title": "Test Project",
        "public": false,
        "labels": [],
        "teams": []
    });

    Mock::given(method("GET"))
        .and(path("/projects/custom%2B123%2Ftest-project"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&project_json))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let server = FossaServer::new(client);

    let command = get_command(serde_json::json!({
        "entity": "project",
        "locator": "custom+123/test-project"
    }))
    .unwrap();
    let result = server
        .handle_get(command)
        .await
        .expect("handle_get should succeed");

    assert!(!result.is_error.unwrap_or(false));
    let text = extract_text(&result);
    assert!(text.contains("Test Project"));
    assert!(text.contains("custom+123/test-project"));
}

#[tokio::test]
async fn test_mcp_get_revision_returns_json() {
    let mock_server = MockServer::start().await;

    let revision_json = serde_json::json!({
        "locator": "custom+123/test$main",
        "resolved": true,
        "sourceType": "cargo"
    });

    Mock::given(method("GET"))
        .and(path("/revisions/custom%2B123%2Ftest%24main"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&revision_json))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let server = FossaServer::new(client);

    let command = get_command(serde_json::json!({
        "entity": "revision",
        "locator": "custom+123/test$main"
    }))
    .unwrap();
    let result = server
        .handle_get(command)
        .await
        .expect("handle_get should succeed");

    assert!(!result.is_error.unwrap_or(false));
    let text = extract_text(&result);
    assert!(text.contains("custom+123/test$main"));
    assert!(text.contains("resolved"));
}

#[tokio::test]
async fn test_mcp_get_issue_returns_json() {
    let mock_server = MockServer::start().await;

    let issue_json = serde_json::json!({
        "id": 12345,
        "type": "vulnerability",
        "source": {"id": "npm+lodash$4.17.0"},
        "depths": {"direct": 1, "deep": 0},
        "statuses": {"active": 1, "ignored": 0},
        "projects": [],
        "cve": "CVE-2024-0001",
        "severity": "high"
    });

    Mock::given(method("GET"))
        .and(path("/v2/issues/12345"))
        .and(query_param("category", "vulnerability"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&issue_json))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let server = FossaServer::new(client);

    let command = get_command(serde_json::json!({
        "entity": "issue",
        "id": 12345,
        "category": "vulnerability"
    }))
    .unwrap();
    let result = server
        .handle_get(command)
        .await
        .expect("handle_get should succeed");

    assert!(!result.is_error.unwrap_or(false));
    let text = extract_text(&result);
    assert!(text.contains("12345"));
    assert!(text.contains("vulnerability"));
    assert!(text.contains("CVE-2024-0001"));
}

/// `dependency` is not a get entity; the schema/deserializer rejects it
/// before any handler runs.
#[test]
fn test_mcp_get_dependency_is_rejected_at_deserialization() {
    let err = get_command(serde_json::json!({
        "entity": "dependency",
        "locator": "npm+lodash$4.17.21"
    }))
    .unwrap_err();
    assert!(
        err.to_string().contains("unknown variant `dependency`"),
        "Error should mention the unknown entity variant: {err}"
    );
}

/// A non-numeric issue id is rejected at deserialization, matching the CLI
/// where the id parses as u64.
#[test]
fn test_mcp_get_issue_with_invalid_id_is_rejected_at_deserialization() {
    let err = get_command(serde_json::json!({
        "entity": "issue",
        "id": "not-a-number",
        "category": "vulnerability"
    }))
    .unwrap_err();
    assert!(
        err.to_string().contains("invalid type: string") && err.to_string().contains("u64"),
        "Error should name the type mismatch (string where u64 expected): {err}"
    );
}

/// The snippet wire format deserializes with its per-entity fields intact.
#[test]
fn test_mcp_get_snippet_wire_format_deserializes() {
    let command = get_command(serde_json::json!({
        "entity": "snippet",
        "revision": "custom+123/test$main",
        "snippet": "55"
    }))
    .unwrap();
    let GetCommand::Snippet(p) = command else {
        panic!("expected Snippet variant");
    };
    assert_eq!(p.revision, "custom+123/test$main");
    assert_eq!(p.snippet, "55");
}

/// The snippet_match wire format deserializes with its per-entity fields intact.
#[test]
fn test_mcp_get_snippet_match_wire_format_deserializes() {
    let command = get_command(serde_json::json!({
        "entity": "snippet_match",
        "revision": "custom+123/test$main",
        "snippet": "55",
        "path": "/src/a.rs"
    }))
    .unwrap();
    let GetCommand::SnippetMatch(p) = command else {
        panic!("expected SnippetMatch variant");
    };
    assert_eq!(p.revision, "custom+123/test$main");
    assert_eq!(p.snippet, "55");
    assert_eq!(p.path, "/src/a.rs");
}

/// Category is optional for get issue: omitting it probes every category,
/// matching the CLI's behavior.
#[test]
fn test_mcp_get_issue_without_category_deserializes() {
    let command = get_command(serde_json::json!({
        "entity": "issue",
        "id": 12345
    }))
    .unwrap();
    assert!(matches!(command, GetCommand::Issue(p) if p.id == 12345 && p.category.is_none()));
}
