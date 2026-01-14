//! Revision endpoint handlers.

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::mock_server::state::MockState;
use crate::Revision;

/// Query parameters for listing revisions.
#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)] // Pagination supported by FOSSA API but not yet used in mock
pub struct ListRevisionsQuery {
    pub page: Option<u32>,
    pub count: Option<u32>,
}

/// Response for listing revisions (grouped by branch).
#[derive(Debug, Serialize)]
pub struct ListRevisionsResponse {
    #[serde(flatten)]
    pub branches: HashMap<String, Vec<Revision>>,
}

/// GET /revisions/{locator}
pub async fn get_revision(
    State(state): State<Arc<RwLock<MockState>>>,
    Path(locator): Path<String>,
) -> impl IntoResponse {
    // URL-decode the locator (handles + and $ encoding)
    let decoded_locator = urlencoding::decode(&locator)
        .map(|s| s.into_owned())
        .unwrap_or(locator);

    let state = state.read().await;

    match state.get_revision(&decoded_locator) {
        Some(revision) => (StatusCode::OK, Json(revision.clone())).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Revision not found",
                "message": format!("No revision found with locator: {}", decoded_locator)
            })),
        )
            .into_response(),
    }
}

/// GET /projects/{locator}/revisions
///
/// Returns revisions grouped by branch.
pub async fn list_revisions(
    State(state): State<Arc<RwLock<MockState>>>,
    Path(project_locator): Path<String>,
    Query(_query): Query<ListRevisionsQuery>,
) -> impl IntoResponse {
    // URL-decode the locator
    let decoded_locator = urlencoding::decode(&project_locator)
        .map(|s| s.into_owned())
        .unwrap_or(project_locator);

    let state = state.read().await;

    let revisions = state.list_revisions_for_project(&decoded_locator);

    // Group revisions by branch
    let mut branches: HashMap<String, Vec<Revision>> = HashMap::new();

    for revision in revisions {
        // Extract branch from locator (format: "project$branch")
        let branch = revision
            .locator
            .rsplit('$')
            .next()
            .unwrap_or("unknown")
            .to_string();

        branches
            .entry(branch)
            .or_default()
            .push(revision.clone());
    }

    (StatusCode::OK, Json(ListRevisionsResponse { branches }))
}
