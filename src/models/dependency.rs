//! Dependency model and trait implementations.

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Supporting Struct Deserialization Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_dependency_status_deserialize() {
        let json = r#"{"error": null, "resolved": true, "unsupported": false, "analyzing": false}"#;
        let status: DependencyStatus = serde_json::from_str(json).expect("Failed to deserialize");
        assert!(status.resolved);
        assert!(!status.unsupported);
        assert!(!status.analyzing);
        assert!(status.error.is_none());
    }

    #[test]
    fn test_dependency_status_deserialize_with_error() {
        let json = r#"{"error": "Failed to resolve", "resolved": false, "unsupported": false, "analyzing": false}"#;
        let status: DependencyStatus = serde_json::from_str(json).expect("Failed to deserialize");
        assert!(!status.resolved);
        assert_eq!(status.error.as_deref(), Some("Failed to resolve"));
    }

    #[test]
    fn test_dependency_status_default() {
        let json = r#"{}"#;
        let status: DependencyStatus = serde_json::from_str(json).expect("Failed to deserialize");
        assert!(!status.resolved);
        assert!(!status.unsupported);
        assert!(!status.analyzing);
    }

    #[test]
    fn test_scoped_conclusion_deserialize() {
        let json = r#"{"licenses": ["MIT", "Apache-2.0"], "lastEditedBy": "user@example.com", "updatedAt": "2024-01-01T00:00:00Z"}"#;
        let scoped: ScopedConclusion = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(scoped.licenses, vec!["MIT", "Apache-2.0"]);
        assert_eq!(scoped.last_edited_by.as_deref(), Some("user@example.com"));
        assert_eq!(scoped.updated_at.as_deref(), Some("2024-01-01T00:00:00Z"));
    }

    #[test]
    fn test_scoped_conclusion_empty() {
        let json = r#"{}"#;
        let scoped: ScopedConclusion = serde_json::from_str(json).expect("Failed to deserialize");
        assert!(scoped.licenses.is_empty());
        assert!(scoped.last_edited_by.is_none());
    }

    #[test]
    fn test_base_conclusion_deserialize() {
        let json = r#"{"licenses": ["BSD-3-Clause"], "justification": "Reviewed and approved"}"#;
        let base: BaseConclusion = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(base.licenses, vec!["BSD-3-Clause"]);
        assert_eq!(base.justification.as_deref(), Some("Reviewed and approved"));
    }

    #[test]
    fn test_base_conclusion_empty() {
        let json = r#"{}"#;
        let base: BaseConclusion = serde_json::from_str(json).expect("Failed to deserialize");
        assert!(base.licenses.is_empty());
        assert!(base.justification.is_none());
    }

    #[test]
    fn test_concluded_licenses_deserialize() {
        let json = r#"{
            "scoped": {"licenses": ["MIT"], "lastEditedBy": "user@example.com"},
            "base": {"licenses": ["MIT"], "justification": "Confirmed"}
        }"#;
        let concluded: ConcludedLicenses = serde_json::from_str(json).expect("Failed to deserialize");
        assert!(concluded.scoped.is_some());
        assert!(concluded.base.is_some());
        assert_eq!(concluded.scoped.as_ref().unwrap().licenses, vec!["MIT"]);
        assert_eq!(concluded.base.as_ref().unwrap().licenses, vec!["MIT"]);
    }

    #[test]
    fn test_concluded_licenses_empty() {
        let json = r#"{}"#;
        let concluded: ConcludedLicenses = serde_json::from_str(json).expect("Failed to deserialize");
        assert!(concluded.scoped.is_none());
        assert!(concluded.base.is_none());
    }

    #[test]
    fn test_dependency_root_project_deserialize() {
        let json = r#"{
            "title": "my-project",
            "revision": "custom+org/my-project$main",
            "branch": "main",
            "conclusions": {"scoped": {"licenses": ["MIT"]}}
        }"#;
        let root: DependencyRootProject =
            serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(root.title.as_deref(), Some("my-project"));
        assert_eq!(
            root.revision.as_deref(),
            Some("custom+org/my-project$main")
        );
        assert_eq!(root.branch.as_deref(), Some("main"));
        assert!(root.conclusions.is_some());
    }

    #[test]
    fn test_dependency_root_project_minimal() {
        let json = r#"{}"#;
        let root: DependencyRootProject =
            serde_json::from_str(json).expect("Failed to deserialize");
        assert!(root.title.is_none());
        assert!(root.revision.is_none());
        assert!(root.branch.is_none());
        assert!(root.conclusions.is_none());
    }

    // -------------------------------------------------------------------------
    // Dependency Model Deserialization Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_dependency_deserialize_with_new_fields() {
        let json = r#"{
            "locator": "npm+lodash$4.17.21",
            "title": "lodash",
            "depth": 1,
            "isManual": false,
            "isIgnored": false,
            "isUnknown": false,
            "licenses": ["MIT"],
            "declaredLicenses": ["MIT"],
            "originPaths": ["/package.json"],
            "packageLabels": ["production"],
            "issues": [],
            "status": {"resolved": true, "unsupported": false, "analyzing": false},
            "concludedLicenses": {"base": {"licenses": ["MIT"]}},
            "rootProjects": [{"title": "my-app", "branch": "main"}],
            "layerDepth": 2,
            "cpes": ["cpe:2.3:a:lodash:lodash:4.17.21:*:*:*:*:*:*:*"],
            "vendoredPaths": [],
            "version": "4.17.21"
        }"#;

        let dep: Dependency = serde_json::from_str(json).expect("Failed to deserialize");

        assert_eq!(dep.locator, "npm+lodash$4.17.21");
        assert!(!dep.is_unknown);
        assert_eq!(dep.declared_licenses, vec!["MIT"]);
        assert!(dep.status.is_some());
        assert!(dep.status.as_ref().unwrap().resolved);
        assert!(dep.concluded_licenses.is_some());
        assert_eq!(dep.root_projects.len(), 1);
        assert_eq!(dep.layer_depth, Some(2));
        assert_eq!(dep.cpes.len(), 1);
        assert!(dep.vendored_paths.is_empty());
        assert_eq!(dep.version_field.as_deref(), Some("4.17.21"));
    }

    #[test]
    fn test_dependency_deserialize_minimal() {
        // Ensure backwards compatibility - only locator required
        let json = r#"{"locator": "npm+test$1.0.0"}"#;
        let dep: Dependency = serde_json::from_str(json).expect("Failed to deserialize");

        assert_eq!(dep.locator, "npm+test$1.0.0");
        assert!(!dep.is_unknown);
        assert!(dep.declared_licenses.is_empty());
        assert!(dep.status.is_none());
        assert!(dep.concluded_licenses.is_none());
        assert!(dep.root_projects.is_empty());
        assert!(dep.layer_depth.is_none());
        assert!(dep.cpes.is_empty());
        assert!(dep.vendored_paths.is_empty());
        assert!(dep.version_field.is_none());
    }

    // -------------------------------------------------------------------------
    // Helper Method Tests
    // -------------------------------------------------------------------------

    fn make_test_dependency() -> Dependency {
        Dependency {
            locator: "npm+test$1.0.0".to_string(),
            title: Some("test".to_string()),
            depth: 1,
            is_manual: false,
            is_ignored: false,
            is_unknown: false,
            licenses: vec![],
            declared_licenses: vec![],
            origin_paths: vec![],
            package_labels: vec![],
            issues: vec![],
            status: None,
            concluded_licenses: None,
            root_projects: vec![],
            layer_depth: None,
            cpes: vec![],
            vendored_paths: vec![],
            version_field: None,
        }
    }

    #[test]
    fn test_dependency_is_resolved() {
        let mut dep = make_test_dependency();
        assert!(!dep.is_resolved()); // No status = not resolved

        dep.status = Some(DependencyStatus {
            resolved: true,
            ..Default::default()
        });
        assert!(dep.is_resolved());
    }

    #[test]
    fn test_dependency_is_analyzing() {
        let mut dep = make_test_dependency();
        assert!(!dep.is_analyzing());

        dep.status = Some(DependencyStatus {
            analyzing: true,
            ..Default::default()
        });
        assert!(dep.is_analyzing());
    }

    #[test]
    fn test_dependency_is_unsupported() {
        let mut dep = make_test_dependency();
        assert!(!dep.is_unsupported());

        dep.status = Some(DependencyStatus {
            unsupported: true,
            ..Default::default()
        });
        assert!(dep.is_unsupported());
    }

    #[test]
    fn test_dependency_concluded_license_ids() {
        let mut dep = make_test_dependency();
        assert!(dep.concluded_license_ids().is_empty());

        dep.concluded_licenses = Some(ConcludedLicenses {
            base: Some(BaseConclusion {
                licenses: vec!["MIT".to_string(), "Apache-2.0".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        });
        assert_eq!(dep.concluded_license_ids(), vec!["MIT", "Apache-2.0"]);
    }

    #[test]
    fn test_dependency_status_error() {
        let mut dep = make_test_dependency();
        assert!(dep.status_error().is_none());

        dep.status = Some(DependencyStatus {
            error: Some("Resolution failed".to_string()),
            ..Default::default()
        });
        assert_eq!(dep.status_error(), Some("Resolution failed"));
    }
}

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

    /// Whether this is an unknown dependency.
    #[serde(default, rename = "isUnknown")]
    pub is_unknown: bool,

    /// Concluded licenses for this dependency.
    #[serde(default, rename = "concludedLicenses")]
    pub concluded_licenses: Option<ConcludedLicenses>,

    /// Declared licenses as simple strings.
    #[serde(default, rename = "declaredLicenses")]
    pub declared_licenses: Vec<String>,

    /// Dependency resolution status.
    #[serde(default)]
    pub status: Option<DependencyStatus>,

    /// Root projects that include this dependency.
    #[serde(default, rename = "rootProjects")]
    pub root_projects: Vec<DependencyRootProject>,

    /// Container layer depth.
    #[serde(default, rename = "layerDepth")]
    pub layer_depth: Option<u32>,

    /// Common Platform Enumeration identifiers.
    #[serde(default)]
    pub cpes: Vec<String>,

    /// Paths where this dependency is vendored.
    #[serde(default, rename = "vendoredPaths")]
    pub vendored_paths: Vec<String>,

    /// Version as a separate field (redundant with locator but provided by API).
    #[serde(default, rename = "version")]
    pub version_field: Option<String>,
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

    /// Whether this dependency has been resolved.
    pub fn is_resolved(&self) -> bool {
        self.status.as_ref().is_some_and(|s| s.resolved)
    }

    /// Whether this dependency is currently being analyzed.
    pub fn is_analyzing(&self) -> bool {
        self.status.as_ref().is_some_and(|s| s.analyzing)
    }

    /// Whether this dependency type is unsupported.
    pub fn is_unsupported(&self) -> bool {
        self.status.as_ref().is_some_and(|s| s.unsupported)
    }

    /// Get the status error message, if any.
    pub fn status_error(&self) -> Option<&str> {
        self.status.as_ref().and_then(|s| s.error.as_deref())
    }

    /// Get the concluded license IDs (from base conclusions).
    pub fn concluded_license_ids(&self) -> Vec<&str> {
        self.concluded_licenses
            .as_ref()
            .and_then(|c| c.base.as_ref())
            .map(|b| b.licenses.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
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

/// Dependency resolution status.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyStatus {
    /// Error message if resolution failed.
    #[serde(default)]
    pub error: Option<String>,

    /// Whether the dependency has been resolved.
    #[serde(default)]
    pub resolved: bool,

    /// Whether the dependency type is unsupported.
    #[serde(default)]
    pub unsupported: bool,

    /// Whether the dependency is currently being analyzed.
    #[serde(default)]
    pub analyzing: bool,
}

/// Scoped license conclusion data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopedConclusion {
    /// The concluded license identifiers.
    #[serde(default)]
    pub licenses: Vec<String>,

    /// User who last edited this conclusion.
    #[serde(default)]
    pub last_edited_by: Option<String>,

    /// When this conclusion was last updated.
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Base license conclusion data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BaseConclusion {
    /// The concluded license identifiers.
    #[serde(default)]
    pub licenses: Vec<String>,

    /// Justification for the license conclusion.
    #[serde(default)]
    pub justification: Option<String>,
}

/// Concluded licenses for a dependency.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConcludedLicenses {
    /// Scoped license conclusions.
    #[serde(default)]
    pub scoped: Option<ScopedConclusion>,

    /// Base license conclusions.
    #[serde(default)]
    pub base: Option<BaseConclusion>,
}

/// A root project that includes a dependency.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyRootProject {
    /// The project title.
    #[serde(default)]
    pub title: Option<String>,

    /// The revision locator.
    #[serde(default)]
    pub revision: Option<String>,

    /// The branch name.
    #[serde(default)]
    pub branch: Option<String>,

    /// License conclusions for this project context.
    #[serde(default)]
    pub conclusions: Option<ConcludedLicenses>,
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
        let path = format!("v2/revisions/{encoded_locator}/dependencies");

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
