//! Issue model and trait implementations.
//!
//! Issues represent vulnerabilities, licensing problems, or quality concerns
//! detected in project dependencies.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::client::FossaClient;
use crate::error::{FossaError, Result};
use crate::pagination::Page;
use crate::traits::{Get, List};

// =============================================================================
// TESTS FIRST (TDD Red Phase)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Model Deserialization Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_issue_deserialize_vulnerability() {
        let json = r#"{
            "id": 27,
            "createdAt": "2023-01-19T22:38:02.961Z",
            "source": {
                "id": "npm+lodash$4.2.0",
                "name": "lodash",
                "url": "https://www.npmjs.com/package/lodash",
                "version": "4.2.0",
                "packageManager": "npm"
            },
            "depths": {"direct": 3, "deep": 0},
            "statuses": {"active": 2, "ignored": 1},
            "projects": [
                {"id": "custom+1/TEST", "status": "active", "depth": 1, "title": "TEST"}
            ],
            "type": "vulnerability",
            "vulnId": "CVE-2018-16487_npm+lodash",
            "title": "General Vulnerability",
            "cve": "CVE-2018-16487",
            "cvss": 9.8,
            "cvssVector": "CVSS:3.0/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H",
            "severity": "critical",
            "details": "A prototype pollution vulnerability was found in lodash.",
            "remediation": {
                "partialFix": "1.15.4",
                "completeFix": "1.16.0",
                "partialFixDistance": "PATCH",
                "completeFixDistance": "MAJOR"
            },
            "cwes": ["CWE-254"],
            "published": "2018-09-04T00:00:00.000Z",
            "exploitability": "MATURE",
            "epss": {"score": 0.1234, "percentile": 0.42}
        }"#;

        let issue: Issue = serde_json::from_str(json).expect("Failed to deserialize vulnerability issue");

        assert_eq!(issue.id, 27);
        assert_eq!(issue.issue_type, "vulnerability");
        assert_eq!(issue.source.id, "npm+lodash$4.2.0");
        assert_eq!(issue.source.name.as_deref(), Some("lodash"));
        assert_eq!(issue.depths.direct, 3);
        assert_eq!(issue.depths.deep, 0);
        assert_eq!(issue.statuses.active, 2);
        assert_eq!(issue.statuses.ignored, 1);
        assert_eq!(issue.projects.len(), 1);
        assert_eq!(issue.vuln_id.as_deref(), Some("CVE-2018-16487_npm+lodash"));
        assert_eq!(issue.cve.as_deref(), Some("CVE-2018-16487"));
        assert_eq!(issue.cvss, Some(9.8));
        assert_eq!(issue.severity.as_deref(), Some("critical"));
        assert_eq!(issue.exploitability.as_deref(), Some("MATURE"));
        assert!(issue.epss.is_some());
        assert_eq!(issue.cwes, vec!["CWE-254"]);
    }

    #[test]
    fn test_issue_deserialize_licensing() {
        let json = r#"{
            "id": 42,
            "createdAt": "2023-02-15T10:00:00.000Z",
            "source": {
                "id": "npm+gpl-package$1.0.0",
                "name": "gpl-package",
                "version": "1.0.0",
                "packageManager": "npm"
            },
            "depths": {"direct": 1, "deep": 2},
            "statuses": {"active": 1, "ignored": 0},
            "projects": [],
            "type": "licensing",
            "license": "GPL-3.0"
        }"#;

        let issue: Issue = serde_json::from_str(json).expect("Failed to deserialize licensing issue");

        assert_eq!(issue.id, 42);
        assert_eq!(issue.issue_type, "licensing");
        assert_eq!(issue.license.as_deref(), Some("GPL-3.0"));
        assert!(issue.cve.is_none());
        assert!(issue.cvss.is_none());
    }

    #[test]
    fn test_issue_deserialize_quality() {
        let json = r#"{
            "id": 100,
            "createdAt": "2023-03-01T08:00:00.000Z",
            "source": {
                "id": "npm+old-package$0.1.0",
                "name": "old-package",
                "version": "0.1.0",
                "packageManager": "npm"
            },
            "depths": {"direct": 0, "deep": 5},
            "statuses": {"active": 1, "ignored": 0},
            "projects": [],
            "type": "quality",
            "qualityRule": {"name": "outdated", "threshold": 365}
        }"#;

        let issue: Issue = serde_json::from_str(json).expect("Failed to deserialize quality issue");

        assert_eq!(issue.id, 100);
        assert_eq!(issue.issue_type, "quality");
        assert!(issue.quality_rule.is_some());
        assert!(issue.license.is_none());
        assert!(issue.cve.is_none());
    }

    #[test]
    fn test_issue_source_deserialize() {
        let json = r#"{
            "id": "npm+lodash$4.2.0",
            "name": "lodash",
            "url": "https://www.npmjs.com/package/lodash",
            "version": "4.2.0",
            "packageManager": "npm"
        }"#;

        let source: IssueSource = serde_json::from_str(json).expect("Failed to deserialize source");

        assert_eq!(source.id, "npm+lodash$4.2.0");
        assert_eq!(source.name.as_deref(), Some("lodash"));
        assert_eq!(source.version.as_deref(), Some("4.2.0"));
        assert_eq!(source.package_manager.as_deref(), Some("npm"));
    }

    #[test]
    fn test_issue_depths_deserialize() {
        let json = r#"{"direct": 3, "deep": 7}"#;
        let depths: IssueDepths = serde_json::from_str(json).expect("Failed to deserialize depths");

        assert_eq!(depths.direct, 3);
        assert_eq!(depths.deep, 7);
    }

    #[test]
    fn test_issue_depths_default() {
        let json = r#"{}"#;
        let depths: IssueDepths = serde_json::from_str(json).expect("Failed to deserialize empty depths");

        assert_eq!(depths.direct, 0);
        assert_eq!(depths.deep, 0);
    }

    #[test]
    fn test_issue_statuses_deserialize() {
        let json = r#"{"active": 5, "ignored": 2}"#;
        let statuses: IssueStatuses = serde_json::from_str(json).expect("Failed to deserialize statuses");

        assert_eq!(statuses.active, 5);
        assert_eq!(statuses.ignored, 2);
    }

    // -------------------------------------------------------------------------
    // Query Serialization Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_issue_list_query_default() {
        let query = IssueListQuery::default();
        let serialized = serde_qs::to_string(&query).expect("Failed to serialize query");

        // Empty query should serialize to empty string (no fields set)
        assert!(serialized.is_empty() || serialized == "");
    }

    #[test]
    fn test_issue_list_query_with_category() {
        let query = IssueListQuery {
            category: Some(IssueCategory::Vulnerability),
            ..Default::default()
        };
        let serialized = serde_qs::to_string(&query).expect("Failed to serialize query");

        assert!(serialized.contains("category=vulnerability"));
    }

    #[test]
    fn test_issue_list_query_with_scope() {
        let query = IssueListQuery {
            scope_type: Some("project".to_string()),
            scope_id: Some("custom+org/project".to_string()),
            ..Default::default()
        };
        let serialized = serde_qs::to_string(&query).expect("Failed to serialize query");

        assert!(serialized.contains("scopeType=project"));
        assert!(serialized.contains("scopeId="));
    }

    #[test]
    fn test_issue_list_query_with_sort() {
        let query = IssueListQuery {
            sort: Some("severity_desc".to_string()),
            ..Default::default()
        };
        let serialized = serde_qs::to_string(&query).expect("Failed to serialize query");

        assert!(serialized.contains("sort=severity_desc"));
    }

    // -------------------------------------------------------------------------
    // Helper Method Tests
    // -------------------------------------------------------------------------

    fn make_test_issue(issue_type: &str) -> Issue {
        Issue {
            id: 1,
            created_at: None,
            issue_type: issue_type.to_string(),
            source: IssueSource {
                id: "npm+test$1.0.0".to_string(),
                name: Some("test".to_string()),
                url: None,
                version: Some("1.0.0".to_string()),
                package_manager: Some("npm".to_string()),
            },
            depths: IssueDepths::default(),
            statuses: IssueStatuses { active: 3, ignored: 1 },
            projects: vec![],
            vuln_id: None,
            title: None,
            cve: Some("CVE-2023-1234".to_string()),
            cvss: Some(7.5),
            cvss_vector: None,
            severity: Some("high".to_string()),
            details: None,
            remediation: None,
            cwes: vec![],
            published: None,
            exploitability: None,
            epss: None,
            license: None,
            quality_rule: None,
        }
    }

    #[test]
    fn test_issue_is_vulnerability() {
        let issue = make_test_issue("vulnerability");
        assert!(issue.is_vulnerability());
        assert!(!issue.is_licensing());
        assert!(!issue.is_quality());
    }

    #[test]
    fn test_issue_is_licensing() {
        let issue = make_test_issue("licensing");
        assert!(!issue.is_vulnerability());
        assert!(issue.is_licensing());
        assert!(!issue.is_quality());
    }

    #[test]
    fn test_issue_is_quality() {
        let issue = make_test_issue("quality");
        assert!(!issue.is_vulnerability());
        assert!(!issue.is_licensing());
        assert!(issue.is_quality());
    }

    #[test]
    fn test_issue_active_count() {
        let issue = make_test_issue("vulnerability");
        assert_eq!(issue.active_count(), 3);
    }

    #[test]
    fn test_issue_ignored_count() {
        let issue = make_test_issue("vulnerability");
        assert_eq!(issue.ignored_count(), 1);
    }

    #[test]
    fn test_issue_source_locator() {
        let issue = make_test_issue("vulnerability");
        assert_eq!(issue.source_locator(), "npm+test$1.0.0");
    }

    #[test]
    fn test_issue_package_name() {
        let issue = make_test_issue("vulnerability");
        assert_eq!(issue.package_name(), Some("test"));
    }

    #[test]
    fn test_issue_package_version() {
        let issue = make_test_issue("vulnerability");
        assert_eq!(issue.package_version(), Some("1.0.0"));
    }

    #[test]
    fn test_issue_severity() {
        let issue = make_test_issue("vulnerability");
        assert_eq!(issue.severity.as_deref(), Some("high"));
    }

    #[test]
    fn test_issue_cve() {
        let issue = make_test_issue("vulnerability");
        assert_eq!(issue.cve.as_deref(), Some("CVE-2023-1234"));
    }

    // -------------------------------------------------------------------------
    // Issue Category Enum Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_issue_category_serialize() {
        assert_eq!(
            serde_json::to_string(&IssueCategory::Vulnerability).unwrap(),
            "\"vulnerability\""
        );
        assert_eq!(
            serde_json::to_string(&IssueCategory::Licensing).unwrap(),
            "\"licensing\""
        );
        assert_eq!(
            serde_json::to_string(&IssueCategory::Quality).unwrap(),
            "\"quality\""
        );
    }

    #[test]
    fn test_issue_category_deserialize() {
        assert!(matches!(
            serde_json::from_str::<IssueCategory>("\"vulnerability\"").unwrap(),
            IssueCategory::Vulnerability
        ));
        assert!(matches!(
            serde_json::from_str::<IssueCategory>("\"licensing\"").unwrap(),
            IssueCategory::Licensing
        ));
        assert!(matches!(
            serde_json::from_str::<IssueCategory>("\"quality\"").unwrap(),
            IssueCategory::Quality
        ));
    }
}

// =============================================================================
// IMPLEMENTATION (TDD Green Phase - to be filled in)
// =============================================================================

/// A FOSSA issue (vulnerability, licensing, or quality).
///
/// Issues are detected problems in project dependencies. They come in three
/// categories:
/// - **Vulnerability**: Security vulnerabilities (CVEs) with severity ratings
/// - **Licensing**: License compliance issues (e.g., GPL in proprietary code)
/// - **Quality**: Code quality concerns (e.g., outdated dependencies)
///
/// # Example
///
/// ```ignore
/// use fossapi::{FossaClient, Issue, IssueListQuery, IssueCategory, List};
///
/// let client = FossaClient::from_env()?;
///
/// // List all vulnerability issues
/// let query = IssueListQuery {
///     category: Some(IssueCategory::Vulnerability),
///     ..Default::default()
/// };
/// let issues = Issue::list_all(&client, &query).await?;
///
/// for issue in issues {
///     if issue.is_vulnerability() {
///         println!("CVE: {:?}, Severity: {:?}", issue.cve, issue.severity);
///     }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// Unique issue ID.
    pub id: u64,

    /// When the issue was first detected.
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,

    /// Issue category: "vulnerability", "licensing", or "quality".
    #[serde(rename = "type")]
    pub issue_type: String,

    /// The source package/dependency where the issue was found.
    pub source: IssueSource,

    /// Depth information (direct vs transitive).
    #[serde(default)]
    pub depths: IssueDepths,

    /// Status counts across projects.
    #[serde(default)]
    pub statuses: IssueStatuses,

    /// Projects affected by this issue.
    #[serde(default)]
    pub projects: Vec<IssueProject>,

    // --- Vulnerability-specific fields ---

    /// Vulnerability ID (e.g., "CVE-2018-16487_npm+lodash").
    #[serde(default)]
    pub vuln_id: Option<String>,

    /// Issue title.
    #[serde(default)]
    pub title: Option<String>,

    /// CVE identifier (e.g., "CVE-2018-16487").
    #[serde(default)]
    pub cve: Option<String>,

    /// CVSS score (0.0 - 10.0).
    #[serde(default)]
    pub cvss: Option<f64>,

    /// CVSS vector string.
    #[serde(default)]
    pub cvss_vector: Option<String>,

    /// Severity level: "critical", "high", "medium", "low".
    #[serde(default)]
    pub severity: Option<String>,

    /// Detailed description of the vulnerability.
    #[serde(default)]
    pub details: Option<String>,

    /// Remediation information (fix versions).
    #[serde(default)]
    pub remediation: Option<IssueRemediation>,

    /// CWE identifiers.
    #[serde(default)]
    pub cwes: Vec<String>,

    /// When the vulnerability was published.
    #[serde(default)]
    pub published: Option<DateTime<Utc>>,

    /// Exploitability: "UNKNOWN", "POC", "MATURE".
    #[serde(default)]
    pub exploitability: Option<String>,

    /// EPSS (Exploit Prediction Scoring System) data.
    #[serde(default)]
    pub epss: Option<IssueEpss>,

    // --- Licensing-specific fields ---

    /// License identifier (e.g., "GPL-3.0").
    #[serde(default)]
    pub license: Option<String>,

    // --- Quality-specific fields ---

    /// Quality rule details.
    #[serde(default)]
    pub quality_rule: Option<serde_json::Value>,
}

impl Issue {
    /// Whether this is a vulnerability issue.
    pub fn is_vulnerability(&self) -> bool {
        self.issue_type == "vulnerability"
    }

    /// Whether this is a licensing issue.
    pub fn is_licensing(&self) -> bool {
        self.issue_type == "licensing"
    }

    /// Whether this is a quality issue.
    pub fn is_quality(&self) -> bool {
        self.issue_type == "quality"
    }

    /// Number of projects where this issue is active.
    pub fn active_count(&self) -> u32 {
        self.statuses.active
    }

    /// Number of projects where this issue is ignored.
    pub fn ignored_count(&self) -> u32 {
        self.statuses.ignored
    }

    /// Get the source package locator.
    pub fn source_locator(&self) -> &str {
        &self.source.id
    }

    /// Get the package name from the source.
    pub fn package_name(&self) -> Option<&str> {
        self.source.name.as_deref()
    }

    /// Get the package version from the source.
    pub fn package_version(&self) -> Option<&str> {
        self.source.version.as_deref()
    }
}

/// Source package information for an issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueSource {
    /// Package locator (e.g., "npm+lodash$4.2.0").
    pub id: String,

    /// Package name.
    #[serde(default)]
    pub name: Option<String>,

    /// Package URL.
    #[serde(default)]
    pub url: Option<String>,

    /// Package version.
    #[serde(default)]
    pub version: Option<String>,

    /// Package manager (e.g., "npm", "maven").
    #[serde(default)]
    pub package_manager: Option<String>,
}

/// Dependency depth information for an issue.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IssueDepths {
    /// Number of direct dependencies affected.
    #[serde(default)]
    pub direct: u32,

    /// Number of transitive (deep) dependencies affected.
    #[serde(default)]
    pub deep: u32,
}

/// Status counts for an issue across projects.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IssueStatuses {
    /// Number of projects where issue is active.
    #[serde(default)]
    pub active: u32,

    /// Number of projects where issue is ignored.
    #[serde(default)]
    pub ignored: u32,
}

/// Project information for an issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueProject {
    /// Project locator.
    pub id: String,

    /// Issue status in this project.
    #[serde(default)]
    pub status: Option<String>,

    /// Dependency depth in this project.
    #[serde(default)]
    pub depth: Option<u32>,

    /// Project title.
    #[serde(default)]
    pub title: Option<String>,
}

/// Remediation information for a vulnerability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueRemediation {
    /// Version that partially fixes the issue.
    #[serde(default)]
    pub partial_fix: Option<String>,

    /// Version that completely fixes the issue.
    #[serde(default)]
    pub complete_fix: Option<String>,

    /// Upgrade distance for partial fix (e.g., "PATCH", "MINOR", "MAJOR").
    #[serde(default)]
    pub partial_fix_distance: Option<String>,

    /// Upgrade distance for complete fix.
    #[serde(default)]
    pub complete_fix_distance: Option<String>,
}

/// EPSS (Exploit Prediction Scoring System) data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueEpss {
    /// EPSS score (probability of exploitation).
    #[serde(default)]
    pub score: Option<f64>,

    /// EPSS percentile ranking.
    #[serde(default)]
    pub percentile: Option<f64>,
}

/// Issue category for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum IssueCategory {
    /// Security vulnerabilities.
    Vulnerability,
    /// License compliance issues.
    Licensing,
    /// Code quality concerns.
    Quality,
}

/// Query parameters for listing issues.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueListQuery {
    /// Filter by issue category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<IssueCategory>,

    /// Filter by status (active, ignored, resolved).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Scope type (project, revision, etc.).
    #[serde(rename = "scopeType", skip_serializing_if = "Option::is_none")]
    pub scope_type: Option<String>,

    /// Scope ID (project/revision locator).
    #[serde(rename = "scopeId", skip_serializing_if = "Option::is_none")]
    pub scope_id: Option<String>,

    /// Sort order (e.g., "severity_desc", "created_at_asc").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
}

/// API response wrapper for issue list.
#[derive(Debug, Deserialize)]
struct IssueListResponse {
    issues: Vec<Issue>,
}

// =============================================================================
// TRAIT IMPLEMENTATIONS
// =============================================================================

#[async_trait]
impl Get for Issue {
    type Id = u64;

    #[tracing::instrument(skip(client))]
    async fn get(client: &FossaClient, id: Self::Id) -> Result<Self> {
        let path = format!("v2/issues/{id}");
        let response = client.get(&path).await?;
        let issue: Issue = response.json().await.map_err(FossaError::HttpError)?;
        Ok(issue)
    }
}

#[async_trait]
impl List for Issue {
    type Query = IssueListQuery;

    #[tracing::instrument(skip(client))]
    async fn list_page(
        client: &FossaClient,
        query: &Self::Query,
        page: u32,
        count: u32,
    ) -> Result<Page<Self>> {
        let path = "v2/issues";

        #[derive(Serialize)]
        struct RequestParams<'a> {
            #[serde(flatten)]
            query: &'a IssueListQuery,
            page: u32,
            count: u32,
        }

        let params = RequestParams { query, page, count };

        let response = client.get_with_query(path, &params).await?;
        let data: IssueListResponse = response.json().await.map_err(FossaError::HttpError)?;

        // Note: Issues API doesn't return total count, so we infer has_more from page size
        Ok(Page::new(data.issues, page, count, None))
    }
}

// =============================================================================
// CONVENIENCE FUNCTIONS
// =============================================================================

/// Fetch all issues matching a query.
///
/// # Arguments
///
/// * `client` - The FOSSA API client
/// * `query` - Query parameters for filtering
///
/// # Example
///
/// ```ignore
/// use fossapi::{FossaClient, get_issues, IssueListQuery, IssueCategory};
///
/// let client = FossaClient::from_env()?;
/// let query = IssueListQuery {
///     category: Some(IssueCategory::Vulnerability),
///     ..Default::default()
/// };
/// let issues = get_issues(&client, query).await?;
/// ```
pub async fn get_issues(client: &FossaClient, query: IssueListQuery) -> Result<Vec<Issue>> {
    Issue::list_all(client, &query).await
}

/// Fetch a single page of issues.
///
/// # Arguments
///
/// * `client` - The FOSSA API client
/// * `query` - Query parameters for filtering
/// * `page` - Page number (1-indexed)
/// * `count` - Number of items per page
pub async fn get_issues_page(
    client: &FossaClient,
    query: IssueListQuery,
    page: u32,
    count: u32,
) -> Result<Page<Issue>> {
    Issue::list_page(client, &query, page, count).await
}

/// Fetch issues for a specific project.
///
/// # Arguments
///
/// * `client` - The FOSSA API client
/// * `project_locator` - The project locator (e.g., "custom+org/project")
/// * `category` - Optional issue category filter
///
/// # Example
///
/// ```ignore
/// use fossapi::{FossaClient, get_project_issues, IssueCategory};
///
/// let client = FossaClient::from_env()?;
/// let issues = get_project_issues(
///     &client,
///     "custom+org/my-project",
///     Some(IssueCategory::Vulnerability),
/// ).await?;
/// ```
pub async fn get_project_issues(
    client: &FossaClient,
    project_locator: &str,
    category: Option<IssueCategory>,
) -> Result<Vec<Issue>> {
    let query = IssueListQuery {
        scope_type: Some("project".to_string()),
        scope_id: Some(project_locator.to_string()),
        category,
        ..Default::default()
    };
    Issue::list_all(client, &query).await
}
