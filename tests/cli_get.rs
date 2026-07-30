//! Execution tests for CLI get command (TDD RED phase)
//!
//! Uses wiremock to mock the FOSSA API and test actual execution flow.

use fossapi::{FossaClient, FossaError, Get, Issue, IssueCategory, Project};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_get_project_returns_json() {
    let mock_server = MockServer::start().await;

    // Minimal Project JSON matching the model's required fields
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
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let project = Project::get(&client, "custom+123/test-project".to_string())
        .await
        .unwrap();

    assert_eq!(project.title, "Test Project");
    assert_eq!(project.locator(), "custom+123/test-project");
}

#[tokio::test]
async fn test_get_calls_trait_method() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/projects/custom%2B123%2Ftest"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "custom+123/test",
                "title": "Test",
                "public": false,
                "labels": [],
                "teams": []
            })),
        )
        .expect(1) // Verify the trait method was called exactly once
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let _ = Project::get(&client, "custom+123/test".to_string()).await;
}

/// A 404 for `category`, mirroring what FOSSA returns when the issue exists
/// under a different category.
async fn mount_category_404(mock_server: &MockServer, category: &str) {
    Mock::given(method("GET"))
        .and(path("/v2/issues/500"))
        .and(query_param("category", category))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "code": 2004,
            "message": "Issue not found",
            "name": "NotFoundError"
        })))
        .mount(mock_server)
        .await;
}

fn issue_json(issue_type: &str) -> serde_json::Value {
    serde_json::json!({
        "id": 500,
        "type": issue_type,
        "source": { "id": "npm+lodash$4.17.0" },
        "depths": { "direct": 1, "deep": 0 },
        "statuses": { "active": 1, "ignored": 0 },
        "projects": []
    })
}

#[tokio::test]
async fn test_get_issue_falls_through_404_to_next_category() {
    let mock_server = MockServer::start().await;

    mount_category_404(&mock_server, "vulnerability").await;
    Mock::given(method("GET"))
        .and(path("/v2/issues/500"))
        .and(query_param("category", "licensing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(issue_json("policy_flag")))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let issue = Issue::get(&client, 500)
        .await
        .expect("Should fall through to the licensing category");

    assert_eq!(issue.id, 500);
    assert_eq!(issue.issue_type, "policy_flag");
}

#[tokio::test]
async fn test_get_issue_not_found_in_any_category() {
    let mock_server = MockServer::start().await;

    for category in ["vulnerability", "licensing", "quality"] {
        mount_category_404(&mock_server, category).await;
    }

    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let err = Issue::get(&client, 500)
        .await
        .expect_err("Should report the issue as missing");

    assert!(
        matches!(err, FossaError::NotFound { entity_type: "Issue", ref id } if id == "500"),
        "Expected NotFound, got: {err:?}"
    );
}

#[tokio::test]
async fn test_get_issue_propagates_non_404_errors() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v2/issues/500"))
        .and(query_param("category", "vulnerability"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "message": "Internal server error"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let err = Issue::get(&client, 500)
        .await
        .expect_err("A 500 should not be swallowed");

    assert!(
        matches!(
            err,
            FossaError::ApiError {
                status_code: Some(500),
                ..
            }
        ),
        "Expected the 500 to propagate, got: {err:?}"
    );
}

#[tokio::test]
async fn test_get_issue_with_category_makes_one_request() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v2/issues/500"))
        .and(query_param("category", "quality"))
        .respond_with(ResponseTemplate::new(200).set_body_json(issue_json("outdated_dependency")))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let issue = Issue::get_with_category(&client, 500, IssueCategory::Quality)
        .await
        .expect("Failed to get issue");

    assert_eq!(issue.issue_type, "outdated_dependency");
}
