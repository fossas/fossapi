# Mock FOSSA Server

Feature-gated (`test-server`) in-memory mock FOSSA API server for E2E testing.

## Purpose

Provides a realistic mock server that maintains state across requests, enabling workflow testing (project → revision → dependencies → issues) beyond wiremock's per-test HTTP mocking.

## Module Structure

```
mock_server/
├── mod.rs           # Public API: MockServer, MockState, Fixtures
├── server.rs        # Axum server setup, routing, lifecycle
├── state.rs         # In-memory data stores (HashMap-based)
├── fixtures.rs      # Test data factories
└── handlers/        # HTTP endpoint handlers
    ├── mod.rs
    ├── projects.rs  # GET/PUT /projects/:locator, GET /v2/projects
    ├── revisions.rs # GET /revisions/:locator, GET /projects/:locator/revisions
    ├── dependencies.rs # GET /v2/revisions/:locator/dependencies
    └── issues.rs    # GET /v2/issues/:id, GET /v2/issues
```

## Usage

```rust
use fossapi::mock_server::{MockServer, MockState, Fixtures};
use fossapi::{FossaClient, Project, Get};

#[tokio::test]
async fn test_workflow() {
    // Start with default fixtures
    let server = MockServer::start().await;

    // Or with custom state
    let state = MockState::new()
        .with_project(Fixtures::minimal_project("custom+1/my-proj", "My Project"));
    let server = MockServer::with_state(state).await;

    let client = FossaClient::new("test-token", server.url()).unwrap();
    // ... tests ...

    server.shutdown().await;
}
```

## API Endpoints

| Endpoint | Method | Handler |
|----------|--------|---------|
| `/projects/:locator` | GET | get_project |
| `/projects/:locator` | PUT | update_project |
| `/v2/projects` | GET | list_projects |
| `/revisions/:locator` | GET | get_revision |
| `/projects/:locator/revisions` | GET | list_revisions |
| `/v2/revisions/:locator/dependencies` | GET | list_dependencies |
| `/v2/issues/:id` | GET | get_issue |
| `/v2/issues` | GET | list_issues |
| `/health` | GET | health_check |

## Running Tests

```bash
cargo test --features test-server
```
