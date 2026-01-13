//! Execution tests for CLI list command (TDD RED phase)
//!
//! Uses wiremock to mock the FOSSA API and test actual execution flow.

use fossapi::{get_dependencies, FossaClient, Issue, List, Project};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_list_projects_returns_page() {
    let mock_server = MockServer::start().await;

    let response = serde_json::json!({
        "projects": [
            {
                "id": "custom+1/proj1",
                "title": "Project 1",
                "public": false,
                "labels": [],
                "teams": []
            },
            {
                "id": "custom+1/proj2",
                "title": "Project 2",
                "public": false,
                "labels": [],
                "teams": []
            }
        ],
        "total": 2
    });

    Mock::given(method("GET"))
        .and(path("/v2/projects"))
        .and(query_param("page", "1"))
        .and(query_param("count", "20"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let page = Project::list_page(&client, &Default::default(), 1, 20)
        .await
        .unwrap();

    assert_eq!(page.items.len(), 2);
    assert_eq!(page.items[0].title, "Project 1");
    assert_eq!(page.items[1].title, "Project 2");
}

#[tokio::test]
async fn test_list_projects_with_pagination() {
    let mock_server = MockServer::start().await;

    let response = serde_json::json!({
        "projects": [
            {
                "id": "custom+1/proj3",
                "title": "Project 3",
                "public": false,
                "labels": [],
                "teams": []
            }
        ],
        "total": 51
    });

    Mock::given(method("GET"))
        .and(path("/v2/projects"))
        .and(query_param("page", "2"))
        .and(query_param("count", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let page = Project::list_page(&client, &Default::default(), 2, 50)
        .await
        .unwrap();

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.page, 2);
    assert_eq!(page.count, 50);
    assert!(!page.has_more);
}

#[tokio::test]
async fn test_list_issues_returns_page() {
    let mock_server = MockServer::start().await;

    let response = serde_json::json!({
        "issues": [
            {
                "id": 1,
                "type": "vulnerability",
                "source": { "id": "npm+lodash$4.17.0" },
                "depths": { "direct": 1, "deep": 0 },
                "statuses": { "active": 1, "ignored": 0 },
                "projects": [],
                "cve": "CVE-2021-1234",
                "severity": "high"
            },
            {
                "id": 2,
                "type": "licensing",
                "source": { "id": "npm+gpl-lib$1.0.0" },
                "depths": { "direct": 0, "deep": 1 },
                "statuses": { "active": 1, "ignored": 0 },
                "projects": [],
                "license": "GPL-3.0"
            }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/v2/issues"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let page = Issue::list_page(&client, &Default::default(), 1, 20)
        .await
        .unwrap();

    assert_eq!(page.items.len(), 2);
    assert_eq!(page.items[0].id, 1);
    assert!(page.items[0].is_vulnerability());
    assert_eq!(page.items[1].id, 2);
    assert!(page.items[1].is_licensing());
}

#[tokio::test]
async fn test_list_dependencies_with_revision() {
    let mock_server = MockServer::start().await;

    let response = serde_json::json!({
        "dependencies": [
            {
                "locator": "npm+lodash$4.17.21",
                "depth": 1,
                "licenses": ["MIT"]
            },
            {
                "locator": "npm+express$4.18.0",
                "depth": 2,
                "licenses": ["MIT"]
            }
        ],
        "count": 2
    });

    Mock::given(method("GET"))
        .and(path("/v2/revisions/custom%2Borg%2Frepo%24abc123/dependencies"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let deps = get_dependencies(&client, "custom+org/repo$abc123", Default::default())
        .await
        .unwrap();

    assert_eq!(deps.len(), 2);
    assert_eq!(deps[0].locator, "npm+lodash$4.17.21");
    assert!(deps[0].is_direct());
    assert_eq!(deps[1].locator, "npm+express$4.18.0");
    assert!(!deps[1].is_direct());
}

#[tokio::test]
async fn test_list_projects_trait_method_called() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v2/projects"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "projects": [],
                "total": 0
            })),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
    let _ = Project::list_page(&client, &Default::default(), 1, 20).await;
}
