//! Team endpoint handlers.
//!
//! Membership is stored on each project's `teams` list, as the real
//! `GET /v2/projects` reports it.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::mock_server::state::MockState;

/// Query parameters for listing teams.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTeamsQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub search: Option<String>,
}

/// Body of a team membership write.
#[derive(Debug, Deserialize)]
pub struct UpdateTeamProjectsBody {
    pub action: String,
    pub projects: Vec<String>,
}

fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "message": message }))).into_response()
}

/// Locators of every project on a team, sorted.
fn team_project_locators(state: &MockState, team_id: u64) -> Vec<String> {
    let mut locators: Vec<String> = state
        .projects
        .values()
        .filter(|p| p.teams.iter().any(|t| t.id == team_id))
        .map(|p| p.id.clone())
        .collect();
    locators.sort();
    locators
}

/// GET /v2/teams
///
/// Like the real route: `pageSize` defaults to 10 and clamps to 50, and the
/// envelope is `{results, page, pageSize, totalCount}`.
pub async fn list_teams(
    State(state): State<Arc<RwLock<MockState>>>,
    Query(query): Query<ListTeamsQuery>,
) -> impl IntoResponse {
    let state = state.read().await;
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(10).clamp(1, 50);

    let mut teams: Vec<_> = state
        .teams
        .values()
        .filter(|t| {
            query
                .search
                .as_deref()
                .map(|s| t.name.to_lowercase().contains(&s.to_lowercase()))
                .unwrap_or(true)
        })
        .cloned()
        .collect();
    teams.sort_by_key(|t| t.id);
    let total_count = teams.len();

    let results: Vec<_> = teams
        .into_iter()
        .skip(((page - 1) * page_size) as usize)
        .take(page_size as usize)
        .map(|mut t| {
            t.team_projects_count = Some(team_project_locators(&state, t.id).len() as u64);
            t
        })
        .collect();

    Json(serde_json::json!({
        "results": results,
        "page": page,
        "pageSize": page_size,
        "totalCount": total_count,
    }))
}

/// GET /projects/{locator}/teams
pub async fn list_project_teams(
    State(state): State<Arc<RwLock<MockState>>>,
    Path(locator): Path<String>,
) -> impl IntoResponse {
    let state = state.read().await;
    let teams: Vec<_> = state
        .get_project(&locator)
        .map(|p| {
            p.teams
                .iter()
                .map(|t| serde_json::json!({ "id": t.id, "name": t.name }))
                .collect()
        })
        .unwrap_or_default();
    Json(serde_json::json!({ "totalCount": teams.len(), "teams": teams }))
}

/// PUT /teams/{id}/projects
///
/// Supports `add` and `remove`. Like the real route: unknown team → 404,
/// and `add` is all-or-nothing → 403 if any locator matches no project
/// (which is what a `$revision` suffix does).
pub async fn update_team_projects(
    State(state): State<Arc<RwLock<MockState>>>,
    Path(team_id): Path<u64>,
    Json(body): Json<UpdateTeamProjectsBody>,
) -> Response {
    let mut state = state.write().await;
    let Some(team) = state.teams.get(&team_id).cloned() else {
        return StatusCode::NOT_FOUND.into_response();
    };

    match body.action.as_str() {
        "add" => {
            if !body.projects.iter().all(|l| state.projects.contains_key(l)) {
                return error(
                    StatusCode::FORBIDDEN,
                    &format!(
                        "You are not permitted to add all selected projects to team \"{}\".",
                        team.name
                    ),
                );
            }
            for locator in &body.projects {
                let project = state.projects.get_mut(locator).expect("checked above");
                if !project.teams.iter().any(|t| t.id == team_id) {
                    project.teams.push(team.clone());
                }
            }
        }
        "remove" => {
            for locator in &body.projects {
                if let Some(project) = state.projects.get_mut(locator) {
                    project.teams.retain(|t| t.id != team_id);
                }
            }
        }
        _ => return error(StatusCode::BAD_REQUEST, "Unsupported action in mock"),
    }

    Json(serde_json::json!({
        "id": team_id,
        "projects": team_project_locators(&state, team_id),
    }))
    .into_response()
}
