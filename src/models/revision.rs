//! Revision model and trait implementations.

use std::collections::HashMap;

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

    /// The parent project ID.
    #[serde(default)]
    pub project_id: Option<String>,

    /// Whether the revision has been resolved/analyzed.
    #[serde(default)]
    pub resolved: bool,

    /// Source type (e.g., "cargo", "npm", "pip").
    #[serde(default)]
    pub source_type: Option<String>,

    /// Analysis source (e.g., "cli", "api").
    #[serde(default)]
    pub source: Option<String>,

    /// Revision message (typically a formatted timestamp).
    #[serde(default)]
    pub message: Option<String>,

    /// Error message if analysis failed.
    #[serde(default)]
    pub error: Option<String>,

    /// When the revision was created/uploaded.
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,

    /// When the revision was last updated.
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,

    /// Revision timestamp string.
    #[serde(default)]
    pub revision_timestamp: Option<String>,

    /// Latest revision scan ID.
    #[serde(default)]
    pub latest_revision_scan_id: Option<u64>,

    /// Count of unresolved issues.
    #[serde(default)]
    pub unresolved_issue_count: Option<u32>,

    /// Structured locator information.
    #[serde(default)]
    pub loc: Option<RevisionLoc>,
}

/// Structured locator information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionLoc {
    /// The fetcher type (e.g., "custom", "git").
    #[serde(default)]
    pub fetcher: Option<String>,
    /// The package identifier.
    #[serde(default)]
    pub package: Option<String>,
    /// The revision identifier.
    #[serde(default)]
    pub revision: Option<String>,
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
        self.loc
            .as_ref()
            .and_then(|l| l.fetcher.as_deref())
            .or_else(|| self.locator.split('+').next())
    }

    /// Check if the revision analysis has completed successfully.
    pub fn is_analyzed(&self) -> bool {
        self.resolved
    }

    /// Check if the revision has any issues.
    pub fn has_issues(&self) -> bool {
        self.unresolved_issue_count.map_or(false, |c| c > 0)
    }

    /// Get the issue count for this revision.
    pub fn issue_count(&self) -> u32 {
        self.unresolved_issue_count.unwrap_or(0)
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

/// Query parameters for listing revisions of a project.
#[derive(Debug, Clone, Default, Serialize)]
pub struct RevisionListQuery {
    /// Filter by branch/ref name (client-side filtering).
    #[serde(skip_serializing)]
    pub branch: Option<String>,
}

/// Query type for revision listing (includes project locator).
pub type RevisionQuery = (String, RevisionListQuery);

/// API response wrapper for listing revisions (grouped by branch).
#[derive(Debug, Deserialize)]
struct RevisionListResponse {
    /// Revisions grouped by branch name.
    #[serde(default)]
    branch: HashMap<String, Vec<Revision>>,
}

#[async_trait]
impl Get for Revision {
    type Id = String; // Revision locator

    #[tracing::instrument(skip(client))]
    async fn get(client: &FossaClient, locator: String) -> Result<Self> {
        let encoded_locator = urlencoding::encode(&locator);
        // Note: Single revision endpoint may differ - using revisions list and filtering
        let path = format!("revisions/{}", encoded_locator);

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
        let path = format!("projects/{}/revisions", encoded_locator);

        // The API returns all revisions grouped by branch (no server-side pagination)
        let response = client.get(&path).await?;
        let data: RevisionListResponse = response.json().await.map_err(FossaError::HttpError)?;

        // Flatten all branches into a single list
        let mut all_revisions: Vec<Revision> = data
            .branch
            .into_iter()
            .filter(|(branch_name, _)| {
                // Apply client-side branch filter if specified
                filters
                    .branch
                    .as_ref()
                    .map_or(true, |filter| branch_name == filter)
            })
            .flat_map(|(_, revisions)| revisions)
            .collect();

        // Sort by created_at descending (newest first)
        all_revisions.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let total = all_revisions.len() as u64;

        // Apply client-side pagination
        let start = ((page - 1) * count) as usize;
        let items: Vec<Revision> = all_revisions.into_iter().skip(start).take(count as usize).collect();

        Ok(Page::new(items, page, count, Some(total)))
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
