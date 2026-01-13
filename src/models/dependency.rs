//! Dependency model and trait implementations.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::client::FossaClient;
use crate::error::{FossaError, Result};
use crate::pagination::Page;
use crate::traits::List;

/// A dependency in a FOSSA project revision.
///
/// Dependencies are the packages that a project depends on, discovered
/// during FOSSA analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    /// The dependency locator (e.g., "npm+react$18.2.0").
    pub locator: String,

    /// The dependency title/name.
    #[serde(default)]
    pub title: Option<String>,

    /// Dependency depth (1 = direct, >1 = transitive).
    #[serde(default)]
    pub depth: u32,

    /// Whether this is a manually added dependency.
    #[serde(default, rename = "isManual")]
    pub is_manual: bool,

    /// Whether this dependency is ignored.
    #[serde(default, rename = "isIgnored")]
    pub is_ignored: bool,

    /// All licenses found for this dependency.
    #[serde(default)]
    pub licenses: Vec<LicenseInfo>,

    /// File paths where this dependency was found.
    #[serde(default, rename = "originPaths")]
    pub origin_paths: Vec<String>,

    /// Package labels (e.g., "production", "development").
    #[serde(default, rename = "packageLabels")]
    pub package_labels: Vec<String>,

    /// Issues associated with this dependency.
    #[serde(default)]
    pub issues: Vec<DependencyIssue>,
}

impl Dependency {
    /// Whether this is a direct dependency.
    pub fn is_direct(&self) -> bool {
        self.depth <= 1
    }

    /// Whether this is a transitive dependency.
    pub fn is_transitive(&self) -> bool {
        self.depth > 1
    }

    /// Whether this dependency has any issues.
    pub fn has_issues(&self) -> bool {
        !self.issues.is_empty()
    }

    /// Get the package name from the locator.
    pub fn package_name(&self) -> Option<&str> {
        // Format: fetcher+package$version
        let after_plus = self.locator.split('+').nth(1)?;
        Some(after_plus.split('$').next().unwrap_or(after_plus))
    }

    /// Get the version from the locator.
    pub fn version(&self) -> Option<&str> {
        self.locator.split('$').nth(1)
    }

    /// Get the fetcher/package manager from the locator.
    pub fn fetcher(&self) -> Option<&str> {
        self.locator.split('+').next()
    }
}

/// License information for a dependency.
///
/// The API can return licenses as either a simple string (e.g., "MIT")
/// or as a full object with id, title, declared, and discovered fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LicenseInfo {
    /// Simple string license (e.g., "MIT", "Apache-2.0").
    Simple(String),
    /// Full license object with metadata.
    Full {
        /// The SPDX license identifier.
        #[serde(default)]
        id: Option<String>,
        /// The license title.
        #[serde(default)]
        title: Option<String>,
        /// Whether this license was declared in the package metadata.
        #[serde(default)]
        declared: bool,
        /// Whether this license was discovered through file analysis.
        #[serde(default)]
        discovered: bool,
    },
}

impl LicenseInfo {
    /// Get the license identifier.
    pub fn id(&self) -> Option<&str> {
        match self {
            LicenseInfo::Simple(s) => Some(s.as_str()),
            LicenseInfo::Full { id, .. } => id.as_deref(),
        }
    }

    /// Get the license title (falls back to id for simple licenses).
    pub fn title(&self) -> Option<&str> {
        match self {
            LicenseInfo::Simple(s) => Some(s.as_str()),
            LicenseInfo::Full { title, id, .. } => title.as_deref().or(id.as_deref()),
        }
    }
}

/// An issue associated with a dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyIssue {
    /// Issue ID.
    pub id: u64,

    /// Issue type.
    #[serde(rename = "type")]
    pub issue_type: IssueType,

    /// Issue status.
    #[serde(default)]
    pub status: IssueStatus,

    /// Severity level (for vulnerabilities).
    #[serde(default)]
    pub severity: Option<String>,

    /// CVE identifier (for vulnerabilities).
    #[serde(default)]
    pub cve: Option<String>,

    /// CVSS score (for vulnerabilities).
    #[serde(rename = "cvssScore", default)]
    pub cvss_score: Option<f64>,
}

/// Type of dependency issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    /// Security vulnerability.
    Vulnerability,
    /// License policy flag.
    PolicyFlag,
    /// License policy conflict.
    PolicyConflict,
    /// Unlicensed dependency.
    UnlicensedDependency,
    /// Outdated dependency.
    OutdatedDependency,
    /// Blacklisted dependency.
    BlacklistedDependency,
    /// Unknown issue type.
    #[serde(other)]
    Unknown,
}

/// Issue status.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueStatus {
    /// Issue is active.
    #[default]
    Active,
    /// Issue has been ignored.
    Ignored,
    /// Issue has been resolved.
    Resolved,
    /// Unknown status.
    #[serde(other)]
    Unknown,
}

/// Query parameters for listing dependencies.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DependencyListQuery {
    /// Filter by title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Include ignored dependencies.
    #[serde(rename = "showIgnored", skip_serializing_if = "Option::is_none")]
    pub show_ignored: Option<bool>,

    /// Filter to direct dependencies only.
    #[serde(rename = "directOnly", skip_serializing_if = "Option::is_none")]
    pub direct_only: Option<bool>,

    /// Filter by package manager/fetcher.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetcher: Option<String>,
}

/// Query type for dependency listing (includes revision locator).
pub type DependencyQuery = (String, DependencyListQuery);

/// API response wrapper for listing dependencies.
#[derive(Debug, Deserialize)]
struct DependencyListResponse {
    dependencies: Vec<Dependency>,
    #[serde(default)]
    count: Option<u64>,
}

#[async_trait]
impl List for Dependency {
    type Query = DependencyQuery; // (revision_locator, filters)

    #[tracing::instrument(skip(client))]
    async fn list_page(
        client: &FossaClient,
        query: &Self::Query,
        page: u32,
        count: u32,
    ) -> Result<Page<Self>> {
        let (revision_locator, filters) = query;
        let encoded_locator = urlencoding::encode(revision_locator);
        let path = format!("v2/revisions/{}/dependencies", encoded_locator);

        #[derive(Serialize)]
        struct RequestParams<'a> {
            #[serde(flatten)]
            query: &'a DependencyListQuery,
            page: u32,
            count: u32,
        }

        let params = RequestParams {
            query: filters,
            page,
            count,
        };

        let response = client.get_with_query(&path, &params).await?;
        let data: DependencyListResponse = response.json().await.map_err(FossaError::HttpError)?;

        Ok(Page::new(data.dependencies, page, count, data.count))
    }
}

// Convenience functions for working with dependencies

/// Fetch all dependencies for a revision.
///
/// # Arguments
///
/// * `client` - The FOSSA API client
/// * `revision_locator` - The revision locator (e.g., "custom+org/project$main")
/// * `query` - Query parameters for filtering
///
/// # Example
///
/// ```ignore
/// use fossapi::{FossaClient, get_dependencies, DependencyListQuery};
///
/// let client = FossaClient::from_env()?;
/// let deps = get_dependencies(
///     &client,
///     "custom+org/project$main",
///     DependencyListQuery::default(),
/// ).await?;
/// ```
pub async fn get_dependencies(
    client: &FossaClient,
    revision_locator: &str,
    query: DependencyListQuery,
) -> Result<Vec<Dependency>> {
    Dependency::list_all(client, &(revision_locator.to_string(), query)).await
}

/// Fetch a single page of dependencies.
///
/// # Arguments
///
/// * `client` - The FOSSA API client
/// * `revision_locator` - The revision locator
/// * `query` - Query parameters for filtering
/// * `page` - Page number (1-indexed)
/// * `count` - Number of items per page
pub async fn get_dependencies_page(
    client: &FossaClient,
    revision_locator: &str,
    query: DependencyListQuery,
    page: u32,
    count: u32,
) -> Result<Page<Dependency>> {
    Dependency::list_page(client, &(revision_locator.to_string(), query), page, count).await
}
