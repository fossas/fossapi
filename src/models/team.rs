//! Team model and team membership operations.
//!
//! A project can sit on any number of teams. The API has no "move" endpoint
//! and adding a project to a team leaves its other teams untouched, so a move
//! is an add to the target followed by a remove from the source.

use serde::{Deserialize, Serialize};

use crate::client::FossaClient;
use crate::error::{FossaError, Result};
use crate::pagination::Page;

/// Hard cap the team endpoints put on `pageSize`.
const MAX_TEAM_PAGE_SIZE: u32 = 50;

/// A FOSSA team.
///
/// `GET /v2/teams` fills every field; the `teams` entries on a project and
/// `GET /projects/{locator}/teams` carry only `id` and `name`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    /// Numeric team ID.
    pub id: u64,

    /// Team name.
    pub name: String,

    /// External identifier (e.g. from SSO group sync), if set.
    #[serde(default, alias = "uniqueId")]
    pub unique_identifier: Option<String>,

    /// Number of projects on the team.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_projects_count: Option<u64>,

    /// Number of members on the team.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_members_count: Option<u64>,
}

/// A team's project membership, as returned by a membership write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamProjects {
    /// The team ID.
    pub id: u64,
    /// Every project locator on the team after the write.
    pub projects: Vec<String>,
}

/// The result of moving projects between teams.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamMove {
    /// The source team's membership after the remove.
    pub from: TeamProjects,
    /// The target team's membership after the add.
    pub to: TeamProjects,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamListResponse {
    results: Vec<Team>,
    page_size: u32,
    total_count: u64,
}

#[derive(Deserialize)]
struct ProjectTeamsResponse {
    teams: Vec<Team>,
}

/// Get one page of teams, optionally filtered by a name search.
///
/// `count` is capped at 50 by the API; the returned page reports the size
/// the API actually used.
#[tracing::instrument(skip(client))]
pub async fn get_teams_page(
    client: &FossaClient,
    search: Option<&str>,
    page: u32,
    count: u32,
) -> Result<Page<Team>> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Params<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        search: Option<&'a str>,
        page: u32,
        page_size: u32,
    }

    let params = Params {
        search,
        page,
        page_size: count.min(MAX_TEAM_PAGE_SIZE),
    };
    let response = client.get_with_query("v2/teams", &params).await?;
    let data: TeamListResponse = response.json().await.map_err(FossaError::HttpError)?;
    Ok(Page::new(
        data.results,
        page,
        data.page_size,
        Some(data.total_count),
    ))
}

/// Get the teams a project is on (only teams visible to the token).
#[tracing::instrument(skip(client))]
pub async fn get_project_teams(client: &FossaClient, project: &str) -> Result<Vec<Team>> {
    let path = format!(
        "projects/{}/teams",
        urlencoding::encode(project_locator(project))
    );
    let response = client.get(&path).await?;
    let data: ProjectTeamsResponse = response.json().await.map_err(FossaError::HttpError)?;
    Ok(data.teams)
}

/// Add projects to a team. Their other team memberships are kept.
///
/// All-or-nothing: the API refuses the whole batch (403) if the token lacks
/// Edit on any of the projects.
pub async fn add_team_projects(
    client: &FossaClient,
    team_id: u64,
    projects: &[String],
) -> Result<TeamProjects> {
    put_team_projects(client, team_id, "add", projects).await
}

/// Remove projects from a team.
pub async fn remove_team_projects(
    client: &FossaClient,
    team_id: u64,
    projects: &[String],
) -> Result<TeamProjects> {
    put_team_projects(client, team_id, "remove", projects).await
}

/// Move projects from one team to another: add to `to`, then remove from
/// `from`. If the add fails nothing is removed; if the remove fails the
/// error says the projects are now on both teams.
pub async fn move_team_projects(
    client: &FossaClient,
    projects: &[String],
    from: u64,
    to: u64,
) -> Result<TeamMove> {
    if from == to {
        // Add would be a no-op and the remove would drop them off the team.
        return Err(FossaError::InvalidParams(format!(
            "cannot move projects from team {from} to itself"
        )));
    }
    if projects.is_empty() {
        // Checked here so the add-error wrapping below can't relabel it.
        return Err(FossaError::InvalidParams(
            "at least one project locator is required".to_string(),
        ));
    }
    let to_team =
        add_team_projects(client, to, projects)
            .await
            .map_err(|e| FossaError::ApiError {
                message: format!(
                    "adding to team {to} failed, so nothing was removed from team {from}: {e}"
                ),
                status_code: None,
            })?;
    let from_team = remove_team_projects(client, from, projects)
        .await
        .map_err(|e| FossaError::ApiError {
            message: format!(
                "added to team {to}, but removing from team {from} failed; \
                 the projects are now on both teams: {e}"
            ),
            status_code: None,
        })?;
    Ok(TeamMove {
        from: from_team,
        to: to_team,
    })
}

/// `PUT /teams/{id}/projects`. Only `add` and `remove` are ever sent: the
/// API's `replace` drops every project on the team not in the list.
#[tracing::instrument(skip(client))]
async fn put_team_projects(
    client: &FossaClient,
    team_id: u64,
    action: &str,
    projects: &[String],
) -> Result<TeamProjects> {
    if projects.is_empty() {
        return Err(FossaError::InvalidParams(
            "at least one project locator is required".to_string(),
        ));
    }
    // A revision suffix makes the locator match no project, which 403s the
    // whole add batch.
    let projects: Vec<&str> = projects.iter().map(|p| project_locator(p)).collect();
    let body = serde_json::json!({ "action": action, "projects": projects });
    let response = client
        .put(&format!("teams/{team_id}/projects"), &body)
        .await?;
    response.json().await.map_err(FossaError::HttpError)
}

/// Strip any `$revision` suffix, leaving the project locator.
fn project_locator(locator: &str) -> &str {
    locator.split('$').next().unwrap_or(locator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_locator_strips_revision() {
        assert_eq!(project_locator("custom+1/repo$abc123"), "custom+1/repo");
        assert_eq!(project_locator("custom+1/repo"), "custom+1/repo");
    }

    #[tokio::test]
    async fn move_reports_both_teams_when_remove_fails() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/teams/2/projects"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"id": 2, "projects": ["custom+1/a"]})),
            )
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .and(path("/teams/1/projects"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let client = FossaClient::new("t", &server.uri()).unwrap();
        let err = move_team_projects(&client, &["custom+1/a".to_string()], 1, 2)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("now on both teams"), "{err}");
    }

    /// `GET /v2/projects` sends `teams` as objects, not strings.
    #[test]
    fn project_team_entry_deserializes() {
        let project: crate::Project = serde_json::from_value(serde_json::json!({
            "id": "custom+1/a",
            "title": "A",
            "teams": [{"id": 7, "name": "Platform", "uniqueId": null, "teamType": "team"}]
        }))
        .unwrap();
        assert_eq!(project.teams[0].id, 7);
        assert_eq!(project.teams[0].name, "Platform");
    }
}
