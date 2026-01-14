//! Project endpoint handlers.

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
use crate::Project;

/// Query parameters for listing projects.
#[derive(Debug, Default, Deserialize)]
pub struct ListProjectsQuery {
    pub page: Option<u32>,
    pub count: Option<u32>,
    pub title: Option<String>,
}

/// Response for listing projects.
#[derive(Debug, Serialize)]
pub struct ListProjectsResponse {
    pub projects: Vec<Project>,
    pub total: u64,
}

/// Parameters for updating a project.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectParams {
    pub title: Option<String>,
    #[allow(dead_code)] // Supported by FOSSA API but not yet used in mock
    pub description: Option<String>,
    pub url: Option<String>,
    pub public: Option<bool>,
}

/// GET /projects/{locator}
pub async fn get_project(
    State(state): State<Arc<RwLock<MockState>>>,
    Path(locator): Path<String>,
) -> impl IntoResponse {
    // URL-decode the locator
    let decoded_locator = urlencoding::decode(&locator)
        .map(|s| s.into_owned())
        .unwrap_or(locator);

    let state = state.read().await;

    match state.get_project(&decoded_locator) {
        Some(project) => (StatusCode::OK, Json(project.clone())).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Project not found",
                "message": format!("No project found with locator: {}", decoded_locator)
            })),
        )
            .into_response(),
    }
}

/// GET /v2/projects
pub async fn list_projects(
    State(state): State<Arc<RwLock<MockState>>>,
    Query(query): Query<ListProjectsQuery>,
) -> impl IntoResponse {
    let state = state.read().await;

    let page = query.page.unwrap_or(1);
    let count = query.count.unwrap_or(20);

    let all_projects = state.list_projects(query.title.as_deref());
    let total = all_projects.len() as u64;

    // Apply pagination
    let start = ((page - 1) * count) as usize;
    let end = (start + count as usize).min(all_projects.len());

    let projects: Vec<Project> = if start < all_projects.len() {
        all_projects[start..end].iter().map(|p| (*p).clone()).collect()
    } else {
        vec![]
    };

    (
        StatusCode::OK,
        Json(ListProjectsResponse { projects, total }),
    )
}

/// PUT /projects/{locator}
pub async fn update_project(
    State(state): State<Arc<RwLock<MockState>>>,
    Path(locator): Path<String>,
    Json(params): Json<UpdateProjectParams>,
) -> impl IntoResponse {
    // URL-decode the locator
    let decoded_locator = urlencoding::decode(&locator)
        .map(|s| s.into_owned())
        .unwrap_or(locator);

    let mut state = state.write().await;

    match state.update_project(&decoded_locator, params.title, params.url, params.public) {
        Some(project) => (StatusCode::OK, Json(project.clone())).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Project not found",
                "message": format!("No project found with locator: {}", decoded_locator)
            })),
        )
            .into_response(),
    }
}
