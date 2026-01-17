//! Tests for MCP get tool handler.
//!
//! Uses wiremock to mock the FOSSA API and test the MCP get tool dispatch.

use fossapi::mcp::{EntityType, FossaServer, GetParams};
use fossapi::{FossaClient, IssueCategory};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper to build GetParams.
fn get_params(entity: EntityType, id: &str, category: Option<IssueCategory>) -> GetParams {
    GetParams {
        entity,
        id: id.to_string(),
        category,
    }
}

/// Extract text from CallToolResult content.
fn extract_text(result: &rmcp::model::CallToolResult) -> &str {
    let content = &result.content[0];
    content.raw.as_text().expect("Expected text content").text.as_str()
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

    let result = server
        .handle_get(get_params(EntityType::Project, "custom+123/test-project", None))
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

    let result = server
        .handle_get(get_params(EntityType::Revision, "custom+123/test$main", None))
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

    let result = server
        .handle_get(get_params(EntityType::Issue, "12345", Some(IssueCategory::Vulnerability)))
        .await
        .expect("handle_get should succeed");

    assert!(!result.is_error.unwrap_or(false));
    let text = extract_text(&result);
    assert!(text.contains("12345"));
    assert!(text.contains("vulnerability"));
    assert!(text.contains("CVE-2024-0001"));
}

#[tokio::test]
async fn test_mcp_get_dependency_returns_error() {
    let mock_server = MockServer::start().await;

    // No mock needed - dependency get should fail before making HTTP request
    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let server = FossaServer::new(client);

    let result = server
        .handle_get(get_params(EntityType::Dependency, "npm+lodash$4.17.21", None))
        .await;

    // Should return an error, not a success
    let err = result.expect_err("get dependency should fail");
    let err_msg = format!("{:?}", err);
    assert!(
        err_msg.contains("does not support get") || err_msg.contains("list with a parent"),
        "Error should mention dependency doesn't support get: {}",
        err_msg
    );
}

#[tokio::test]
async fn test_mcp_get_issue_with_invalid_id_returns_error() {
    let mock_server = MockServer::start().await;

    // No mock needed - parsing should fail before HTTP request
    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let server = FossaServer::new(client);

    let result = server
        .handle_get(get_params(EntityType::Issue, "not-a-number", Some(IssueCategory::Vulnerability)))
        .await;

    let err = result.expect_err("get issue with invalid ID should fail");
    let err_msg = format!("{:?}", err);
    assert!(
        err_msg.contains("must be a number"),
        "Error should mention issue ID must be numeric: {}",
        err_msg
    );
}

#[tokio::test]
async fn test_mcp_get_issue_without_category_returns_error() {
    let mock_server = MockServer::start().await;

    // No mock needed - should fail before HTTP request
    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let server = FossaServer::new(client);

    let result = server
        .handle_get(get_params(EntityType::Issue, "12345", None))
        .await;

    let err = result.expect_err("get issue without category should fail");
    let err_msg = format!("{:?}", err);
    assert!(
        err_msg.to_lowercase().contains("category"),
        "Error should mention category is required: {}",
        err_msg
    );
}
