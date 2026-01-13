//! Execution tests for CLI get command (TDD RED phase)
//!
//! Uses wiremock to mock the FOSSA API and test actual execution flow.

use fossapi::{FossaClient, Get, Project};
use wiremock::matchers::{method, path};
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

    // wiremock verifies the expectation on MockServer drop
}
