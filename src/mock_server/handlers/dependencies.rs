//! Dependency endpoint handlers.

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
use crate::Dependency;

/// Query parameters for listing dependencies.
#[derive(Debug, Default, Deserialize)]
pub struct ListDependenciesQuery {
    pub page: Option<u32>,
    pub count: Option<u32>,
}

/// Response for listing dependencies.
#[derive(Debug, Serialize)]
pub struct ListDependenciesResponse {
    pub dependencies: Vec<Dependency>,
    pub count: u64,
}

/// GET /v2/revisions/{locator}/dependencies
pub async fn list_dependencies(
    State(state): State<Arc<RwLock<MockState>>>,
    Path(revision_locator): Path<String>,
    Query(query): Query<ListDependenciesQuery>,
) -> impl IntoResponse {
    // URL-decode the locator
    let decoded_locator = urlencoding::decode(&revision_locator)
        .map(|s| s.into_owned())
        .unwrap_or(revision_locator);

    let state = state.read().await;

    let page = query.page.unwrap_or(1);
    let count = query.count.unwrap_or(100);

    match state.get_dependencies(&decoded_locator) {
        Some(all_deps) => {
            let total = all_deps.len() as u64;

            // Apply pagination
            let start = ((page - 1) * count) as usize;
            let end = (start + count as usize).min(all_deps.len());

            let dependencies: Vec<Dependency> = if start < all_deps.len() {
                all_deps[start..end].to_vec()
            } else {
                vec![]
            };

            (
                StatusCode::OK,
                Json(ListDependenciesResponse {
                    dependencies,
                    count: total,
                }),
            )
                .into_response()
        }
        None => {
            // Return empty list if no dependencies found for this revision
            (
                StatusCode::OK,
                Json(ListDependenciesResponse {
                    dependencies: vec![],
                    count: 0,
                }),
            )
                .into_response()
        }
    }
}
