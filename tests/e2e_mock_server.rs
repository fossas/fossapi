//! E2E tests using the mock FOSSA server.
//!
//! These tests exercise full workflows against the mock server,
//! testing realistic scenarios rather than individual endpoints.

#![cfg(feature = "test-server")]

use fossapi::mock_server::{Fixtures, MockServer, MockState};
use fossapi::{
    get_dependencies, FossaClient, Get, Issue, List, Project, Revision, Update,
};

// =============================================================================
// Server Lifecycle Tests
// =============================================================================

#[tokio::test]
async fn test_server_starts_on_random_port() {
    let server1 = MockServer::start().await;
    let server2 = MockServer::start().await;

    // Both servers should have different URLs
    assert_ne!(server1.url(), server2.url());

    server1.shutdown().await;
    server2.shutdown().await;
}

#[tokio::test]
async fn test_server_shutdown_is_clean() {
    let server = MockServer::start().await;
    let url = server.url().to_string();

    server.shutdown().await;

    // After shutdown, server should not respond
    let client = reqwest::Client::new();
    let result = client.get(format!("{}/health", url)).send().await;

    assert!(result.is_err());
}

// =============================================================================
// Project Workflow Tests
// =============================================================================

#[tokio::test]
async fn test_list_and_get_project_workflow() {
    let server = MockServer::start().await;
    let client = FossaClient::new("test-token", server.url()).unwrap();

    // Step 1: List all projects
    let page = Project::list_page(&client, &Default::default(), 1, 20)
        .await
        .expect("Failed to list projects");

    assert!(!page.items.is_empty(), "Expected at least one project");

    // Step 2: Get the first project by its locator
    let first_project = &page.items[0];
    let project = Project::get(&client, first_project.id.clone())
        .await
        .expect("Failed to get project");

    assert_eq!(project.id, first_project.id);
    assert_eq!(project.title, first_project.title);

    server.shutdown().await;
}

#[tokio::test]
async fn test_update_project_workflow() {
    let server = MockServer::start().await;
    let client = FossaClient::new("test-token", server.url()).unwrap();

    let locator = "custom+1/test-project".to_string();

    // Step 1: Get original project
    let original = Project::get(&client, locator.clone())
        .await
        .expect("Failed to get project");

    assert_eq!(original.title, "Test Project");

    // Step 2: Update the project
    let update_params = fossapi::ProjectUpdateParams {
        title: Some("Updated Project Title".to_string()),
        public: Some(true),
        ..Default::default()
    };

    let updated = Project::update(&client, locator.clone(), update_params)
        .await
        .expect("Failed to update project");

    assert_eq!(updated.title, "Updated Project Title");
    assert!(updated.public);

    // Step 3: Verify update persisted
    let fetched = Project::get(&client, locator)
        .await
        .expect("Failed to get updated project");

    assert_eq!(fetched.title, "Updated Project Title");
    assert!(fetched.public);

    server.shutdown().await;
}

#[tokio::test]
async fn test_project_not_found() {
    let server = MockServer::start().await;
    let client = FossaClient::new("test-token", server.url()).unwrap();

    let result = Project::get(&client, "nonexistent/project".to_string()).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("not found") || err_str.contains("404"),
        "Error should indicate not found: {}",
        err_str
    );

    server.shutdown().await;
}

// =============================================================================
// Revision Workflow Tests
// =============================================================================

#[tokio::test]
async fn test_get_revision_for_project() {
    let server = MockServer::start().await;
    let client = FossaClient::new("test-token", server.url()).unwrap();

    let revision_locator = "custom+1/test-project$main".to_string();

    let revision = Revision::get(&client, revision_locator.clone())
        .await
        .expect("Failed to get revision");

    assert_eq!(revision.locator, revision_locator);
    assert!(revision.resolved);

    server.shutdown().await;
}

// =============================================================================
// Dependency Workflow Tests
// =============================================================================

#[tokio::test]
async fn test_list_dependencies_for_revision() {
    let server = MockServer::start().await;
    let client = FossaClient::new("test-token", server.url()).unwrap();

    let revision_locator = "custom+1/test-project$main";

    let deps = get_dependencies(&client, revision_locator, Default::default())
        .await
        .expect("Failed to list dependencies");

    assert!(!deps.is_empty(), "Expected dependencies in default fixture");

    // Check we have both direct and transitive deps
    let direct_deps: Vec<_> = deps.iter().filter(|d| d.is_direct()).collect();
    let transitive_deps: Vec<_> = deps.iter().filter(|d| !d.is_direct()).collect();

    assert!(
        !direct_deps.is_empty(),
        "Expected at least one direct dependency"
    );
    assert!(
        !transitive_deps.is_empty(),
        "Expected at least one transitive dependency"
    );

    server.shutdown().await;
}

// =============================================================================
// Issue Workflow Tests
// =============================================================================

#[tokio::test]
async fn test_list_and_get_issues() {
    let server = MockServer::start().await;
    let client = FossaClient::new("test-token", server.url()).unwrap();

    // Step 1: List all issues
    let page = Issue::list_page(&client, &Default::default(), 1, 20)
        .await
        .expect("Failed to list issues");

    assert!(!page.items.is_empty(), "Expected issues in default fixture");

    // Step 2: Get a specific issue
    let first_issue = &page.items[0];
    let issue = Issue::get(&client, first_issue.id)
        .await
        .expect("Failed to get issue");

    assert_eq!(issue.id, first_issue.id);
    assert_eq!(issue.issue_type, first_issue.issue_type);

    server.shutdown().await;
}

#[tokio::test]
async fn test_issues_have_correct_types() {
    let server = MockServer::start().await;
    let client = FossaClient::new("test-token", server.url()).unwrap();

    let page = Issue::list_page(&client, &Default::default(), 1, 100)
        .await
        .expect("Failed to list issues");

    // Default fixture should have both vulnerability and licensing issues
    let vuln_issues: Vec<_> = page.items.iter().filter(|i| i.is_vulnerability()).collect();
    let license_issues: Vec<_> = page.items.iter().filter(|i| i.is_licensing()).collect();

    assert!(!vuln_issues.is_empty(), "Expected vulnerability issues");
    assert!(!license_issues.is_empty(), "Expected licensing issues");

    // Vulnerability issues should have CVE
    for issue in vuln_issues {
        assert!(issue.cve.is_some(), "Vulnerability should have CVE");
    }

    // Licensing issues should have license
    for issue in license_issues {
        assert!(issue.license.is_some(), "Licensing issue should have license");
    }

    server.shutdown().await;
}

// =============================================================================
// Full Workflow Tests
// =============================================================================

#[tokio::test]
async fn test_full_project_analysis_workflow() {
    let server = MockServer::start().await;
    let client = FossaClient::new("test-token", server.url()).unwrap();

    // This test simulates a typical user workflow:
    // 1. List projects to find one
    // 2. Get project details
    // 3. Get the latest revision
    // 4. List dependencies for that revision
    // 5. Check for issues

    // Step 1: List projects
    let projects = Project::list_page(&client, &Default::default(), 1, 10)
        .await
        .expect("Failed to list projects");
    assert!(!projects.items.is_empty());

    let project = &projects.items[0];

    // Step 2: Get project details
    let project_detail = Project::get(&client, project.id.clone())
        .await
        .expect("Failed to get project");

    // Step 3: Get revision (using the latest_revision info if available)
    if let Some(latest_rev) = &project_detail.latest_revision {
        let revision = Revision::get(&client, latest_rev.locator.clone())
            .await
            .expect("Failed to get revision");
        assert!(revision.resolved);

        // Step 4: List dependencies
        let deps = get_dependencies(&client, &revision.locator, Default::default())
            .await
            .expect("Failed to list dependencies");
        assert!(!deps.is_empty());
    }

    // Step 5: Check for issues
    let issues = Issue::list_page(&client, &Default::default(), 1, 100)
        .await
        .expect("Failed to list issues");
    // Issues exist in our test data
    assert!(!issues.items.is_empty());

    server.shutdown().await;
}

// =============================================================================
// Custom State Tests
// =============================================================================

#[tokio::test]
async fn test_custom_state_with_multiple_projects() {
    let state = MockState::new()
        .with_project(Fixtures::minimal_project("custom+org/alpha", "Alpha Project"))
        .with_project(Fixtures::minimal_project("custom+org/beta", "Beta Project"))
        .with_project(Fixtures::project_with_issues(
            "custom+org/gamma",
            "Gamma Project",
            5,
            3,
            2,
        ));

    let server = MockServer::with_state(state).await;
    let client = FossaClient::new("test-token", server.url()).unwrap();

    let page = Project::list_page(&client, &Default::default(), 1, 100)
        .await
        .expect("Failed to list projects");

    assert_eq!(page.items.len(), 3);

    // Get the project with issues
    let gamma = Project::get(&client, "custom+org/gamma".to_string())
        .await
        .expect("Failed to get gamma project");

    let issues = gamma.issues.expect("Gamma should have issues");
    assert_eq!(issues.total, 10);
    assert_eq!(issues.security, 5);
    assert_eq!(issues.licensing, 3);
    assert_eq!(issues.quality, 2);

    server.shutdown().await;
}

#[tokio::test]
async fn test_empty_server_returns_empty_lists() {
    let server = MockServer::start_empty().await;
    let client = FossaClient::new("test-token", server.url()).unwrap();

    let projects = Project::list_page(&client, &Default::default(), 1, 100)
        .await
        .expect("Failed to list projects");

    assert!(projects.items.is_empty());
    assert_eq!(projects.total, Some(0));

    let issues = Issue::list_page(&client, &Default::default(), 1, 100)
        .await
        .expect("Failed to list issues");

    assert!(issues.items.is_empty());

    server.shutdown().await;
}

// =============================================================================
// URL Encoding Tests
// =============================================================================

#[tokio::test]
async fn test_locator_with_special_characters() {
    // Locators contain + and $ which need proper URL encoding
    let state = MockState::new()
        .with_project(Fixtures::minimal_project(
            "custom+58216/github.com/fossas/test-repo",
            "Test Repo",
        ))
        .with_revision(Fixtures::resolved_revision(
            "custom+58216/github.com/fossas/test-repo$feature/branch-name",
            "npm",
        ));

    let server = MockServer::with_state(state).await;
    let client = FossaClient::new("test-token", server.url()).unwrap();

    // Test project with + in locator
    let project = Project::get(
        &client,
        "custom+58216/github.com/fossas/test-repo".to_string(),
    )
    .await
    .expect("Failed to get project with + in locator");

    assert_eq!(project.title, "Test Repo");

    // Test revision with $ in locator
    let revision = Revision::get(
        &client,
        "custom+58216/github.com/fossas/test-repo$feature/branch-name".to_string(),
    )
    .await
    .expect("Failed to get revision with $ in locator");

    assert!(revision.resolved);

    server.shutdown().await;
}
