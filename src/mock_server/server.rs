//! Mock FOSSA API server.
//!
//! Provides an axum-based HTTP server that simulates the FOSSA API.

use std::sync::Arc;

use axum::{
    routing::{get, put},
    Router,
};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use super::fixtures::{DefaultScenario, Fixtures};
use super::handlers;
use super::state::MockState;

/// A mock FOSSA API server for testing.
///
/// The server runs in the background and can be used to test the FOSSA client
/// against a realistic API implementation.
pub struct MockServer {
    /// The URL where the server is listening.
    url: String,
    /// Handle to the server task.
    handle: JoinHandle<()>,
    /// Shared state that can be modified during tests.
    state: Arc<RwLock<MockState>>,
}

impl MockServer {
    /// Start a new mock server with default fixtures.
    ///
    /// The server listens on a random available port and returns immediately.
    /// Use `url()` to get the server's base URL.
    pub async fn start() -> Self {
        Self::with_state(Self::default_state()).await
    }

    /// Start a mock server with empty state.
    ///
    /// Useful when you want to control exactly what data is available.
    pub async fn start_empty() -> Self {
        Self::with_state(MockState::new()).await
    }

    /// Start a mock server with custom state.
    pub async fn with_state(state: MockState) -> Self {
        let shared_state = state.shared();
        let app = Self::create_router(shared_state.clone());

        // Bind to a random available port
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind to address");
        let addr = listener.local_addr().expect("Failed to get local address");

        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("Server error");
        });

        Self {
            url: format!("http://{}", addr),
            handle,
            state: shared_state,
        }
    }

    /// Get the base URL of the mock server.
    ///
    /// Use this URL when creating a `FossaClient` for testing.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get access to the server's shared state.
    ///
    /// This allows modifying the mock data during a test.
    pub fn state(&self) -> Arc<RwLock<MockState>> {
        self.state.clone()
    }

    /// Shutdown the server.
    ///
    /// This aborts the server task. It's safe to call multiple times.
    pub async fn shutdown(self) {
        self.handle.abort();
        let _ = self.handle.await;
    }

    /// Create the default state with common test fixtures.
    fn default_state() -> MockState {
        let scenario = Fixtures::default_scenario();
        Self::state_from_scenario(scenario)
    }

    /// Create state from a scenario.
    fn state_from_scenario(scenario: DefaultScenario) -> MockState {
        let mut state = MockState::new();

        for project in scenario.projects {
            state.projects.insert(project.id.clone(), project);
        }

        for revision in scenario.revisions {
            state.revisions.insert(revision.locator.clone(), revision);
        }

        for (revision_locator, deps) in scenario.dependencies {
            state.dependencies.insert(revision_locator, deps);
        }

        for issue in scenario.issues {
            state.issues.insert(issue.id, issue);
        }

        state
    }

    /// Create the axum router with all routes.
    fn create_router(state: Arc<RwLock<MockState>>) -> Router {
        Router::new()
            // Project routes
            .route("/projects/:locator", get(handlers::get_project))
            .route("/projects/:locator", put(handlers::update_project))
            .route("/v2/projects", get(handlers::list_projects))
            // Revision routes
            .route("/revisions/:locator", get(handlers::get_revision))
            .route(
                "/projects/:locator/revisions",
                get(handlers::list_revisions),
            )
            // Dependency routes
            .route(
                "/v2/revisions/:locator/dependencies",
                get(handlers::list_dependencies),
            )
            // Issue routes
            .route("/v2/issues/:id", get(handlers::get_issue))
            .route("/v2/issues", get(handlers::list_issues))
            // Health check
            .route("/health", get(health_check))
            .with_state(state)
    }
}

/// Health check endpoint.
async fn health_check() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FossaClient, Get, List, Project};

    #[tokio::test]
    async fn test_server_starts_and_responds() {
        let server = MockServer::start().await;

        // Server should be accessible
        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/health", server.url()))
            .send()
            .await
            .expect("Failed to send request");

        assert!(response.status().is_success());
        assert_eq!(response.text().await.unwrap(), "ok");

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_get_project_with_fossa_client() {
        let server = MockServer::start().await;
        let client = FossaClient::new("test-token", server.url()).unwrap();

        let project = Project::get(&client, "custom+1/test-project".to_string())
            .await
            .expect("Failed to get project");

        assert_eq!(project.title, "Test Project");

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_list_projects_with_fossa_client() {
        let server = MockServer::start().await;
        let client = FossaClient::new("test-token", server.url()).unwrap();

        let page = Project::list_page(&client, &Default::default(), 1, 20)
            .await
            .expect("Failed to list projects");

        assert!(!page.items.is_empty());
        assert_eq!(page.items[0].title, "Test Project");

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_empty_server() {
        let server = MockServer::start_empty().await;
        let client = FossaClient::new("test-token", server.url()).unwrap();

        let result = Project::get(&client, "nonexistent".to_string()).await;

        assert!(result.is_err());

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_custom_state() {
        let state = MockState::new().with_project(Fixtures::minimal_project(
            "custom+test/my-project",
            "My Custom Project",
        ));

        let server = MockServer::with_state(state).await;
        let client = FossaClient::new("test-token", server.url()).unwrap();

        let project = Project::get(&client, "custom+test/my-project".to_string())
            .await
            .expect("Failed to get project");

        assert_eq!(project.title, "My Custom Project");

        server.shutdown().await;
    }
}
