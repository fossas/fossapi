//! Project model and trait implementations.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::client::FossaClient;
use crate::error::{FossaError, Result};
use crate::pagination::Page;
use crate::traits::{Get, List, Update};

/// A FOSSA project.
///
/// Projects are the top-level containers for analyzed code. Each project
/// can have multiple revisions (snapshots of the project at different points
/// in time).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    /// The project ID/locator (e.g., "custom+58216/github.com/fossas/repo").
    /// Note: The list endpoint returns "id" while the get endpoint returns "locator".
    #[serde(alias = "locator")]
    pub id: String,

    /// The project title.
    pub title: String,

    /// Default branch for this project.
    #[serde(default)]
    pub branch: Option<String>,

    /// Project version.
    #[serde(default)]
    pub version: Option<String>,

    /// Project type (e.g., "provided", "git").
    #[serde(rename = "type", default)]
    pub project_type: Option<String>,

    /// Project URL.
    #[serde(default)]
    pub url: Option<String>,

    /// Whether the project is public.
    #[serde(default)]
    pub public: bool,

    /// When the project was last scanned.
    #[serde(default)]
    pub scanned: Option<DateTime<Utc>>,

    /// When the project was last analyzed.
    #[serde(rename = "lastAnalyzed", default)]
    pub last_analyzed: Option<DateTime<Utc>>,

    /// Issue counts by category.
    #[serde(default)]
    pub issues: Option<ProjectIssues>,

    /// Project labels.
    #[serde(default)]
    pub labels: Vec<String>,

    /// Teams associated with this project.
    #[serde(default)]
    pub teams: Vec<String>,

    /// Latest revision information.
    #[serde(rename = "latestRevision", default)]
    pub latest_revision: Option<LatestRevision>,

    /// Latest build status.
    #[serde(rename = "latestBuildStatus", default)]
    pub latest_build_status: Option<String>,
}

/// Issue counts for a project.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectIssues {
    /// Total number of issues.
    #[serde(default)]
    pub total: u32,
    /// Number of licensing issues.
    #[serde(default)]
    pub licensing: u32,
    /// Number of security issues.
    #[serde(default)]
    pub security: u32,
    /// Number of quality issues.
    #[serde(default)]
    pub quality: u32,
}

/// Latest revision information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestRevision {
    /// The revision locator.
    pub locator: String,
    /// Revision message/description.
    #[serde(default)]
    pub message: Option<String>,
}

impl Project {
    /// Get the project locator (alias for id).
    pub fn locator(&self) -> &str {
        &self.id
    }

    /// Get the fetcher type from the locator (e.g., "custom", "git").
    pub fn fetcher(&self) -> Option<&str> {
        self.id.split('+').next()
    }

    /// Check if this project has been analyzed.
    pub fn is_analyzed(&self) -> bool {
        self.latest_revision.is_some()
    }

    /// Get the latest revision locator if available.
    pub fn latest_revision_locator(&self) -> Option<&str> {
        self.latest_revision.as_ref().map(|r| r.locator.as_str())
    }

    /// Get all revisions for this project.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let project = Project::get(&client, "custom+org/project".to_string()).await?;
    /// let revisions = project.revisions(&client).await?;
    /// for rev in revisions {
    ///     println!("Revision: {} - {:?}", rev.locator, rev.status);
    /// }
    /// ```
    pub async fn revisions(
        &self,
        client: &FossaClient,
    ) -> Result<Vec<crate::models::revision::Revision>> {
        crate::models::revision::get_revisions(
            client,
            &self.id,
            crate::models::revision::RevisionListQuery::default(),
        )
        .await
    }

    /// Get all revisions with custom query filters.
    pub async fn revisions_with_query(
        &self,
        client: &FossaClient,
        query: crate::models::revision::RevisionListQuery,
    ) -> Result<Vec<crate::models::revision::Revision>> {
        crate::models::revision::get_revisions(client, &self.id, query).await
    }

    /// Get the latest revision as a full Revision model.
    ///
    /// Returns `None` if the project has no revisions.
    pub async fn get_latest_revision(
        &self,
        client: &FossaClient,
    ) -> Result<Option<crate::models::revision::Revision>> {
        match &self.latest_revision {
            Some(lr) => {
                let revision =
                    crate::models::revision::get_revision(client, &lr.locator).await?;
                Ok(Some(revision))
            }
            None => Ok(None),
        }
    }
}

/// Query parameters for listing projects.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ProjectListQuery {
    /// Filter by project title (partial match).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Sort order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
}

/// Parameters for updating a project.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectUpdateParams {
    /// New project title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// New project description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// New project URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Whether the project is public.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public: Option<bool>,

    /// Policy ID to apply.
    #[serde(rename = "policyId", skip_serializing_if = "Option::is_none")]
    pub policy_id: Option<u64>,

    /// Default branch.
    #[serde(rename = "defaultBranch", skip_serializing_if = "Option::is_none")]
    pub default_branch: Option<String>,
}

/// API response wrapper for listing projects.
#[derive(Debug, Deserialize)]
struct ProjectListResponse {
    projects: Vec<Project>,
    #[serde(default)]
    total: Option<u64>,
}

#[async_trait]
impl Get for Project {
    type Id = String; // Project locator

    #[tracing::instrument(skip(client))]
    async fn get(client: &FossaClient, locator: String) -> Result<Self> {
        let encoded_locator = urlencoding::encode(&locator);
        let path = format!("projects/{}", encoded_locator);

        let response = client.get(&path).await?;
        let project: Project = response.json().await.map_err(FossaError::HttpError)?;
        Ok(project)
    }
}

#[async_trait]
impl List for Project {
    type Query = ProjectListQuery;

    #[tracing::instrument(skip(client))]
    async fn list_page(
        client: &FossaClient,
        query: &Self::Query,
        page: u32,
        count: u32,
    ) -> Result<Page<Self>> {
        #[derive(Serialize)]
        struct RequestParams<'a> {
            #[serde(flatten)]
            query: &'a ProjectListQuery,
            page: u32,
            count: u32,
        }

        let params = RequestParams {
            query,
            page,
            count,
        };

        let response = client.get_with_query("v2/projects", &params).await?;
        let data: ProjectListResponse = response.json().await.map_err(FossaError::HttpError)?;

        Ok(Page::new(data.projects, page, count, data.total))
    }
}

#[async_trait]
impl Update for Project {
    type Id = String; // Project locator
    type Params = ProjectUpdateParams;

    #[tracing::instrument(skip(client))]
    async fn update(client: &FossaClient, locator: String, params: Self::Params) -> Result<Self> {
        let encoded_locator = urlencoding::encode(&locator);
        let path = format!("projects/{}", encoded_locator);

        let response = client.put(&path, &params).await?;
        let project: Project = response.json().await.map_err(FossaError::HttpError)?;
        Ok(project)
    }
}
