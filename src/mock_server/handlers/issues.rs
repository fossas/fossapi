//! Issue endpoint handlers.

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
use crate::Issue;

/// Query parameters for getting a single issue.
#[derive(Debug, Default, Deserialize)]
pub struct GetIssueQuery {
    pub category: Option<String>,
}

/// Query parameters for listing issues.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListIssuesQuery {
    pub page: Option<u32>,
    pub count: Option<u32>,
    pub category: Option<String>,
    #[allow(dead_code)] // Supported by FOSSA API but not yet used in mock
    pub scope_type: Option<String>,
    #[allow(dead_code)] // Supported by FOSSA API but not yet used in mock
    pub scope_id: Option<String>,
}

/// Response for listing issues.
#[derive(Debug, Serialize)]
pub struct ListIssuesResponse {
    pub issues: Vec<Issue>,
}

/// GET /v2/issues/{id}
pub async fn get_issue(
    State(state): State<Arc<RwLock<MockState>>>,
    Path(id): Path<String>,
    Query(query): Query<GetIssueQuery>,
) -> impl IntoResponse {
    let Some(category) = query.category.as_deref() else {
        return missing_category();
    };

    let id = match id.parse::<u64>() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid issue ID",
                    "message": "Issue ID must be a number"
                })),
            )
                .into_response()
        }
    };

    let state = state.read().await;

    match state.get_issue_in_category(id, category) {
        Some(issue) => (StatusCode::OK, Json(issue.clone())).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Issue not found",
                "message": format!("No {} issue found with ID: {}", category, id)
            })),
        )
            .into_response(),
    }
}

/// The 400 the FOSSA API returns when the required `category` param is absent.
fn missing_category() -> axum::response::Response {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": "Validation error",
            "message": "Invalid option: expected one of \"licensing\"|\"vulnerability\"|\"quality\" at \"category\""
        })),
    )
        .into_response()
}

/// GET /v2/issues
pub async fn list_issues(
    State(state): State<Arc<RwLock<MockState>>>,
    Query(query): Query<ListIssuesQuery>,
) -> impl IntoResponse {
    let Some(category) = query.category.as_deref() else {
        return missing_category();
    };

    let state = state.read().await;

    let page = query.page.unwrap_or(1);
    let count = query.count.unwrap_or(20);

    let all_issues = state.list_issues(Some(category));

    // Apply pagination
    let start = ((page - 1) * count) as usize;
    let end = (start + count as usize).min(all_issues.len());

    let issues: Vec<Issue> = if start < all_issues.len() {
        all_issues[start..end].iter().map(|i| (*i).clone()).collect()
    } else {
        vec![]
    };

    (StatusCode::OK, Json(ListIssuesResponse { issues })).into_response()
}
