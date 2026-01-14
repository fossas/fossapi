//! Mock FOSSA API server for E2E testing.
//!
//! This module provides an in-memory mock server that simulates the FOSSA API
//! for integration and end-to-end testing. Unlike wiremock which mocks at the
//! HTTP level per-test, this server maintains state across requests, enabling
//! realistic workflow testing.
//!
//! # Example
//!
//! ```ignore
//! use fossapi::mock_server::MockServer;
//! use fossapi::{FossaClient, Project, Get};
//!
//! #[tokio::test]
//! async fn test_workflow() {
//!     let server = MockServer::start().await;
//!     let client = FossaClient::new("test-token", &server.url()).unwrap();
//!
//!     // Server comes with default fixtures
//!     let project = Project::get(&client, "custom+1/test".to_string()).await.unwrap();
//!     assert_eq!(project.title, "Test Project");
//!
//!     server.shutdown().await;
//! }
//! ```

mod fixtures;
mod handlers;
mod server;
mod state;

pub use fixtures::Fixtures;
pub use server::MockServer;
pub use state::MockState;
