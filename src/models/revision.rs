//! Revision model and trait implementations.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::client::FossaClient;
use crate::error::{FossaError, Result};
use crate::pagination::Page;
use crate::traits::{Get, List};

/// A FOSSA project revision.
///
/// Revisions represent snapshots of a project at a specific point in time,
/// typically corresponding to a branch, tag, or commit. Each revision has
/// its own set of dependencies and issue scan results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Revision {
    /// The revision locator (e.g., "custom+58216/github.com/org/repo$main").
    pub locator: String,

    /// The branch, tag, or commit reference.
    #[serde(default, alias = "branch")]
    pub ref_name: Option<String>,

    /// Revision message (typically commit message).
    #[serde(default)]
    pub message: Option<String>,

    /// When the revision was created/uploaded.
    #[serde(rename = "createdAt", default)]
    pub created_at: Option<DateTime<Utc>>,

    /// When the revision was last updated.
    #[serde(rename = "updatedAt", default)]
    pub updated_at: Option<DateTime<Utc>>,

    /// Build/analysis status.
    #[serde(default)]
    pub status: Option<RevisionStatus>,

    /// Issue counts for this revision.
    #[serde(default)]
    pub issues: Option<RevisionIssues>,

    /// Dependency statistics.
    #[serde(default)]
    pub stats: Option<RevisionStats>,

    /// Whether this is the default/latest revision.
    #[serde(rename = "isDefault", default)]
    pub is_default: bool,

    /// Policy ID applied to this revision.
    #[serde(rename = "policyId", default)]
    pub policy_id: Option<u64>,
}

impl Revision {
    /// Get the project locator from the revision locator.
    ///
    /// Revision locators have format: `fetcher+org/project$ref`
    /// This extracts `fetcher+org/project`.
    pub fn project_locator(&self) -> Option<&str> {
        self.locator.split('$').next()
    }

    /// Get the ref (branch/tag/commit) from the locator.
    pub fn ref_from_locator(&self) -> Option<&str> {
        self.locator.split('$').nth(1)
    }

    /// Get the fetcher type from the locator (e.g., "custom", "git").
    pub fn fetcher(&self) -> Option<&str> {
        self.locator.split('+').next()
    }

    /// Check if the revision analysis has completed successfully.
    pub fn is_analyzed(&self) -> bool {
        matches!(self.status, Some(RevisionStatus::Passed))
    }

    /// Check if the revision has any issues.
    pub fn has_issues(&self) -> bool {
        self.issues.as_ref().map_or(false, |i| i.total > 0)
    }

    /// Get direct dependency count.
    pub fn direct_dependency_count(&self) -> u32 {
        self.stats.as_ref().map_or(0, |s| s.direct_dependencies)
    }

    /// Get total dependency count.
    pub fn total_dependency_count(&self) -> u32 {
        self.stats.as_ref().map_or(0, |s| s.total_dependencies)
    }

    /// Get all dependencies for this revision.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let revision = get_revision(&client, "custom+org/project$main").await?;
    /// let deps = revision.dependencies(&client).await?;
    /// for dep in deps {
    ///     println!("  {} (depth: {})", dep.locator, dep.depth);
    /// }
    /// ```
    pub async fn dependencies(
        &self,
        client: &FossaClient,
    ) -> Result<Vec<crate::models::dependency::Dependency>> {
        crate::models::dependency::get_dependencies(
            client,
            &self.locator,
            crate::models::dependency::DependencyListQuery::default(),
        )
        .await
    }

    /// Get dependencies with custom query filters.
    pub async fn dependencies_with_query(
        &self,
        client: &FossaClient,
        query: crate::models::dependency::DependencyListQuery,
    ) -> Result<Vec<crate::models::dependency::Dependency>> {
        crate::models::dependency::get_dependencies(client, &self.locator, query).await
    }
}

/// Status of a revision's analysis.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RevisionStatus {
    /// Analysis is pending.
    #[default]
    Pending,
    /// Analysis is running.
    Running,
    /// Analysis completed successfully.
    Passed,
    /// Analysis failed.
    Failed,
    /// Analysis was skipped.
    Skipped,
    /// Unknown status.
    #[serde(other)]
    Unknown,
}

/// Issue counts for a revision.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionIssues {
    /// Total number of issues.
    #[serde(default)]
    pub total: u32,
    /// Number of licensing issues.
    #[serde(default)]
    pub licensing: u32,
    /// Number of security/vulnerability issues.
    #[serde(default)]
    pub security: u32,
    /// Number of quality issues.
    #[serde(default)]
    pub quality: u32,
}

/// Dependency statistics for a revision.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionStats {
    /// Total dependency count.
    #[serde(default)]
    pub total_dependencies: u32,
    /// Direct dependency count.
    #[serde(default)]
    pub direct_dependencies: u32,
    /// Transitive dependency count.
    #[serde(default)]
    pub transitive_dependencies: u32,
}

/// Query parameters for listing revisions of a project.
#[derive(Debug, Clone, Default, Serialize)]
pub struct RevisionListQuery {
    /// Filter by branch/ref name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// Filter by status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<RevisionStatus>,

    /// Sort order (e.g., "created_at", "-created_at").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,

    /// Include only default/latest revisions per branch.
    #[serde(rename = "defaultOnly", skip_serializing_if = "Option::is_none")]
    pub default_only: Option<bool>,
}

/// Query type for revision listing (includes project locator).
pub type RevisionQuery = (String, RevisionListQuery);

/// API response wrapper for listing revisions.
#[derive(Debug, Deserialize)]
struct RevisionListResponse {
    revisions: Vec<Revision>,
    #[serde(default)]
    total: Option<u64>,
}

#[async_trait]
impl Get for Revision {
    type Id = String; // Revision locator

    #[tracing::instrument(skip(client))]
    async fn get(client: &FossaClient, locator: String) -> Result<Self> {
        let encoded_locator = urlencoding::encode(&locator);
        let path = format!("v2/revisions/{}", encoded_locator);

        let response = client.get(&path).await?;
        let revision: Revision = response.json().await.map_err(FossaError::HttpError)?;
        Ok(revision)
    }
}

#[async_trait]
impl List for Revision {
    type Query = RevisionQuery; // (project_locator, filters)

    #[tracing::instrument(skip(client))]
    async fn list_page(
        client: &FossaClient,
        query: &Self::Query,
        page: u32,
        count: u32,
    ) -> Result<Page<Self>> {
        let (project_locator, filters) = query;
        let encoded_locator = urlencoding::encode(project_locator);
        let path = format!("v2/projects/{}/revisions", encoded_locator);

        #[derive(Serialize)]
        struct RequestParams<'a> {
            #[serde(flatten)]
            query: &'a RevisionListQuery,
            page: u32,
            count: u32,
        }

        let params = RequestParams {
            query: filters,
            page,
            count,
        };

        let response = client.get_with_query(&path, &params).await?;
        let data: RevisionListResponse = response.json().await.map_err(FossaError::HttpError)?;

        Ok(Page::new(data.revisions, page, count, data.total))
    }
}

// Convenience functions for working with revisions

/// Fetch all revisions for a project.
///
/// # Arguments
///
/// * `client` - The FOSSA API client
/// * `project_locator` - The project locator (e.g., "custom+org/project")
/// * `query` - Query parameters for filtering
///
/// # Example
///
/// ```ignore
/// use fossapi::{FossaClient, get_revisions, RevisionListQuery};
///
/// let client = FossaClient::from_env()?;
/// let revisions = get_revisions(
///     &client,
///     "custom+org/project",
///     RevisionListQuery::default(),
/// ).await?;
/// ```
pub async fn get_revisions(
    client: &FossaClient,
    project_locator: &str,
    query: RevisionListQuery,
) -> Result<Vec<Revision>> {
    Revision::list_all(client, &(project_locator.to_string(), query)).await
}

/// Fetch a single page of revisions.
///
/// # Arguments
///
/// * `client` - The FOSSA API client
/// * `project_locator` - The project locator
/// * `query` - Query parameters for filtering
/// * `page` - Page number (1-indexed)
/// * `count` - Number of items per page
pub async fn get_revisions_page(
    client: &FossaClient,
    project_locator: &str,
    query: RevisionListQuery,
    page: u32,
    count: u32,
) -> Result<Page<Revision>> {
    Revision::list_page(client, &(project_locator.to_string(), query), page, count).await
}

/// Get a single revision by locator.
///
/// # Arguments
///
/// * `client` - The FOSSA API client
/// * `revision_locator` - The revision locator (e.g., "custom+org/project$main")
///
/// # Example
///
/// ```ignore
/// use fossapi::{FossaClient, get_revision};
///
/// let client = FossaClient::from_env()?;
/// let revision = get_revision(&client, "custom+org/project$main").await?;
/// println!("Revision status: {:?}", revision.status);
/// ```
pub async fn get_revision(client: &FossaClient, revision_locator: &str) -> Result<Revision> {
    Revision::get(client, revision_locator.to_string()).await
}
