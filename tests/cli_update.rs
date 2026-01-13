//! Execution tests for CLI update command (TDD RED phase)
//!
//! Uses wiremock to mock the FOSSA API and test actual execution flow.

use fossapi::{FossaClient, Project, ProjectUpdateParams, Update};
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// =============================================================================
// TDD Tests for ISS-10845: UpdateCommand execution
// =============================================================================

#[tokio::test]
async fn test_update_project_returns_updated_entity() {
    let mock_server = MockServer::start().await;

    // Expected request body
    let expected_params = serde_json::json!({
        "title": "Updated Title",
        "public": true
    });

    // Response with updated project
    let updated_project = serde_json::json!({
        "id": "custom+acme/myapp",
        "title": "Updated Title",
        "public": true,
        "labels": [],
        "teams": []
    });

    Mock::given(method("PUT"))
        .and(path("/projects/custom%2Bacme%2Fmyapp"))
        .and(body_json(&expected_params))
        .respond_with(ResponseTemplate::new(200).set_body_json(&updated_project))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let params = ProjectUpdateParams {
        title: Some("Updated Title".to_string()),
        public: Some(true),
        ..Default::default()
    };

    let project = Project::update(&client, "custom+acme/myapp".to_string(), params)
        .await
        .unwrap();

    assert_eq!(project.title, "Updated Title");
    assert!(project.public);
    assert_eq!(project.locator(), "custom+acme/myapp");
}
