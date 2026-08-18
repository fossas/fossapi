//! Issue model and trait implementations.
//!
//! Issues represent vulnerabilities, licensing problems, or quality concerns
//! detected in project dependencies.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use clap::ValueEnum;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::client::FossaClient;
use crate::error::{FossaError, Result};
use crate::pagination::Page;
use crate::traits::{Get, List, Update};

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
            "epss": {"score": 0.1234, "percentile": 0.42},
            "url": "https://app.fossa.com/issues/vulnerability/27",
            "cveStatus": "COMPLETED",
            "affectedVersionRanges": ["<5.0.52", ">=5.1.0-beta.0,<5.1.0-beta.9"],
            "patchedVersionRanges": [],
            "cpes": [],
            "references": [
                "https://github.com/vercel/ai/commit/930399bb9839a8baf3d349614106d78268775eed",
                "https://vercel.com/changelog/cve-2025-48985-input-validation-bypass-on-ai-sdk"
            ],
            "metrics": [
                {"name": "Attack Vector", "value": "Network"},
                {"name": "Attack Complexity", "value": "High"}
            ]
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
        assert_eq!(
            issue.url.as_deref(),
            Some("https://app.fossa.com/issues/vulnerability/27")
        );
        assert_eq!(issue.cve_status.as_deref(), Some("COMPLETED"));
        assert_eq!(
            issue.affected_version_ranges,
            vec!["<5.0.52", ">=5.1.0-beta.0,<5.1.0-beta.9"]
        );
        assert!(issue.patched_version_ranges.is_empty());
        assert_eq!(issue.references.len(), 2);
        assert!(issue.references[0].contains("github.com/vercel/ai/commit"));
        assert_eq!(issue.metrics.len(), 2);
        assert_eq!(issue.metrics[0].name, "Attack Vector");
        assert_eq!(issue.metrics[0].value.as_deref(), Some("Network"));

        let remediation = issue
            .remediation
            .expect("Vulnerability should have remediation");
        assert_eq!(remediation.partial_fix.as_deref(), Some("1.15.4"));
        assert_eq!(remediation.complete_fix.as_deref(), Some("1.16.0"));
        assert_eq!(remediation.partial_fix_distance.as_deref(), Some("PATCH"));
        assert_eq!(remediation.complete_fix_distance.as_deref(), Some("MAJOR"));
    }

    /// Guards the camelCase mapping: every key here is one the API actually
    /// sends, and each is `Option` + `#[serde(default)]`, so a rename would
    /// silently deserialize to `None` rather than fail.
    #[test]
    fn test_issue_remediation_deserialize_all_fields() {
        let json = r#"{
            "partialFix": "5.0.52",
            "completeFix": "6.0.0",
            "partialFixDistance": "MINOR",
            "completeFixDistance": "MAJOR"
        }"#;

        let remediation =
            serde_json::from_str::<IssueRemediation>(json).expect("Failed to deserialize");

        assert_eq!(remediation.partial_fix.as_deref(), Some("5.0.52"));
        assert_eq!(remediation.complete_fix.as_deref(), Some("6.0.0"));
        assert_eq!(remediation.partial_fix_distance.as_deref(), Some("MINOR"));
        assert_eq!(remediation.complete_fix_distance.as_deref(), Some("MAJOR"));
    }

    #[test]
    fn test_issue_without_remediation_deserializes() {
        let json = r#"{
            "id": 28,
            "type": "vulnerability",
            "source": {"id": "npm+lodash$4.2.0"},
            "depths": {"direct": 1, "deep": 0},
            "statuses": {"active": 1, "ignored": 0},
            "projects": []
        }"#;

        let issue = serde_json::from_str::<Issue>(json).expect("Failed to deserialize");

        assert!(issue.remediation.is_none());
    }

    /// A metric with no `value` must not fail the whole issue's deserialization.
    #[test]
    fn test_issue_metric_deserialize() {
        let metric =
            serde_json::from_str::<IssueMetric>(r#"{"name": "Attack Vector", "value": "Network"}"#)
                .expect("Failed to deserialize metric");
        assert_eq!(metric.name, "Attack Vector");
        assert_eq!(metric.value.as_deref(), Some("Network"));

        let valueless = serde_json::from_str::<IssueMetric>(r#"{"name": "Scope"}"#)
            .expect("Failed to deserialize valueless metric");
        assert!(valueless.value.is_none());
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
            "license": "GPL-3.0",
            "url": "https://app.fossa.com/issues/licensing/42"
        }"#;

        let issue: Issue = serde_json::from_str(json).expect("Failed to deserialize licensing issue");

        assert_eq!(issue.id, 42);
        assert_eq!(issue.issue_type, "licensing");
        assert_eq!(issue.license.as_deref(), Some("GPL-3.0"));
        assert!(issue.cve.is_none());
        assert!(issue.cvss.is_none());
        assert_eq!(
            issue.url.as_deref(),
            Some("https://app.fossa.com/issues/licensing/42")
        );
        assert!(issue.affected_version_ranges.is_empty());
        assert!(issue.references.is_empty());
        assert!(issue.metrics.is_empty());
        assert!(issue.cve_status.is_none());
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
            "qualityRule": {"name": "outdated", "threshold": 365},
            "latestVersion": "npm+old-package$2.0.0",
            "url": "https://app.fossa.com/issues/quality/100"
        }"#;

        let issue: Issue = serde_json::from_str(json).expect("Failed to deserialize quality issue");

        assert_eq!(issue.id, 100);
        assert_eq!(issue.issue_type, "quality");
        assert!(issue.quality_rule.is_some());
        assert!(issue.license.is_none());
        assert!(issue.cve.is_none());
        assert_eq!(
            issue.latest_version.as_deref(),
            Some("npm+old-package$2.0.0")
        );
        assert_eq!(
            issue.url.as_deref(),
            Some("https://app.fossa.com/issues/quality/100")
        );
    }

    /// Every key the API sends on a project entry, including its three timestamp formats.
    #[test]
    fn test_issue_project_deserialize_full() {
        let json = r#"{
            "id": "custom+58216/testproject/withslash",
            "title": "testproject/withslash",
            "status": "active",
            "depth": 1,
            "url": "https://app.fossa.com/projects/custom%2B58216%2Ftestproject%2Fwithslash",
            "revisionId": "custom+58216/testproject/withslash$2026-04-10T16:08:51Z",
            "revisionScanId": 114469956,
            "defaultBranch": "master",
            "latest": true,
            "firstFoundAt": "2026-04-10T16:19:48.2+00:00",
            "scannedAt": "2026-04-10T16:19:50.611168+00:00",
            "analyzedAt": "2026-04-10T16:09:30.488Z"
        }"#;

        let project =
            serde_json::from_str::<IssueProject>(json).expect("Failed to deserialize issue project");

        assert_eq!(project.id, "custom+58216/testproject/withslash");
        assert_eq!(project.status.as_deref(), Some("active"));
        assert!(project.url.is_some());
        assert_eq!(
            project.revision_id.as_deref(),
            Some("custom+58216/testproject/withslash$2026-04-10T16:08:51Z")
        );
        assert_eq!(project.revision_scan_id, Some(114469956));
        assert_eq!(project.default_branch.as_deref(), Some("master"));
        assert_eq!(project.latest, Some(true));
        assert!(project.first_found_at.is_some());
        assert!(project.scanned_at.is_some());
        assert!(project.analyzed_at.is_some());
    }

    /// Fields absent on a minimal project entry default rather than erroring.
    #[test]
    fn test_issue_project_deserialize_minimal() {
        let project = serde_json::from_str::<IssueProject>(r#"{"id": "custom+1/TEST"}"#)
            .expect("Failed to deserialize minimal issue project");

        assert_eq!(project.id, "custom+1/TEST");
        assert!(project.url.is_none());
        assert!(project.latest.is_none());
        assert!(project.first_found_at.is_none());
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
            url: None,
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
            affected_version_ranges: vec![],
            patched_version_ranges: vec![],
            references: vec![],
            metrics: vec![],
            cve_status: None,
            license: None,
            quality_rule: None,
            latest_version: None,
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

    #[test]
    fn test_issue_action_ignore_serializes_full() {
        let action = IssueAction::Ignore {
            notes: Some("false positive patch".to_string()),
            reason: Some(IssueIgnoreReason::VulnerableCodeNotInExecutePath),
        };
        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "ignore",
                "notes": "false positive patch",
                "reason": "Vulnerable_code_not_in_execute_path"
            })
        );
    }

    #[test]
    fn test_issue_action_ignore_omits_empty_fields() {
        let action = IssueAction::Ignore {
            notes: None,
            reason: None,
        };
        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json, serde_json::json!({"type": "ignore"}));
    }

    #[test]
    fn test_issue_action_unignore_serializes() {
        let json = serde_json::to_value(&IssueAction::Unignore).unwrap();
        assert_eq!(json, serde_json::json!({"type": "unignore"}));
    }

    #[test]
    fn test_issue_ignore_reason_api_strings() {
        // The API matches these strings against its ResolutionReasons table;
        // a mismatch silently records no reason, so pin every variant.
        let cases = [
            (IssueIgnoreReason::Fixed, "Fixed"),
            (IssueIgnoreReason::UnderInvestigation, "Under_investigation"),
            (IssueIgnoreReason::IncorrectDataFound, "incorrect_data_found"),
            (IssueIgnoreReason::ComponentNotPresent, "Component_not_present"),
            (
                IssueIgnoreReason::VulnerableCodeNotPresent,
                "Vulnerable_code_not_present",
            ),
            (
                IssueIgnoreReason::VulnerableCodeNotInExecutePath,
                "Vulnerable_code_not_in_execute_path",
            ),
            (
                IssueIgnoreReason::VulnerableCodeCannotBeControlledByAdversary,
                "Vulnerable_code_cannot_be_controlled_by_adversary",
            ),
            (
                IssueIgnoreReason::InlineMitigationsAlreadyExist,
                "Inline_mitigations_already_exist",
            ),
            (IssueIgnoreReason::Other, "other"),
        ];
        for (reason, expected) in cases {
            assert_eq!(
                serde_json::to_value(reason).unwrap(),
                serde_json::json!(expected)
            );
        }
    }

    #[test]
    fn test_issue_action_response_single() {
        let json = r#"{"count": 1, "issueId": 987654}"#;
        let resp: IssueActionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.count, 1);
        assert_eq!(resp.issue_id, Some(987654));
    }

    #[test]
    fn test_issue_action_response_batch_has_no_issue_id() {
        let json = r#"{"count": 42}"#;
        let resp: IssueActionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.count, 42);
        assert_eq!(resp.issue_id, None);
    }

    #[test]
    fn test_issue_category_as_str() {
        assert_eq!(IssueCategory::Vulnerability.as_str(), "vulnerability");
        assert_eq!(IssueCategory::Licensing.as_str(), "licensing");
        assert_eq!(IssueCategory::Quality.as_str(), "quality");
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

    /// Deep link to this issue in the FOSSA UI. Present on all three categories.
    #[serde(default)]
    pub url: Option<String>,

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

    /// Version ranges known to be vulnerable (e.g. `["<5.0.52", ">=5.1.0-beta.0,<5.1.0-beta.9"]`).
    #[serde(default)]
    pub affected_version_ranges: Vec<String>,

    /// Version ranges carrying the fix. Populated far less often than
    /// [`Issue::affected_version_ranges`]; prefer [`Issue::remediation`] for upgrade targets.
    #[serde(default)]
    pub patched_version_ranges: Vec<String>,

    /// Upstream advisory links: fix commits, vendor changelogs, CVE records.
    #[serde(default)]
    pub references: Vec<String>,

    /// CVSS vector decomposed into readable name/value pairs.
    #[serde(default)]
    pub metrics: Vec<IssueMetric>,

    /// State of FOSSA's CVE enrichment (e.g. "COMPLETED"). Anything other than
    /// completed explains missing `cvss`/`severity`.
    #[serde(default)]
    pub cve_status: Option<String>,

    // --- Licensing-specific fields ---

    /// License identifier (e.g., "GPL-3.0").
    #[serde(default)]
    pub license: Option<String>,

    // --- Quality-specific fields ---

    /// Quality rule details.
    #[serde(default)]
    pub quality_rule: Option<serde_json::Value>,

    /// Locator of the newest published version of the package (e.g. "npm+abab$2.0.6").
    #[serde(default)]
    pub latest_version: Option<String>,
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

    /// Fetch an issue by ID with category.
    ///
    /// The FOSSA API requires a category parameter when fetching issues.
    ///
    /// # Arguments
    ///
    /// * `client` - The FOSSA API client
    /// * `id` - The issue ID
    /// * `category` - The issue category (vulnerability, licensing, quality)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use fossapi::{FossaClient, Issue, IssueCategory};
    ///
    /// let client = FossaClient::from_env()?;
    /// let issue = Issue::get_with_category(&client, 12345, IssueCategory::Vulnerability).await?;
    /// ```
    pub async fn get_with_category(
        client: &FossaClient,
        id: u64,
        category: IssueCategory,
    ) -> Result<Self> {
        #[derive(Serialize)]
        struct Query {
            category: IssueCategory,
        }
        let path = format!("v2/issues/{id}");
        let response = client.get_with_query(&path, &Query { category }).await?;
        let issue: Issue = response.json().await.map_err(FossaError::HttpError)?;
        Ok(issue)
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

    /// Deep link to the project in the FOSSA UI.
    #[serde(default)]
    pub url: Option<String>,

    /// Locator of the revision where the issue was found.
    #[serde(default)]
    pub revision_id: Option<String>,

    /// Whether that revision is the project's latest.
    #[serde(default)]
    pub latest: Option<bool>,

    /// Numeric ID of the scan that produced the revision.
    #[serde(default)]
    pub revision_scan_id: Option<u64>,

    /// The project's default branch.
    #[serde(default)]
    pub default_branch: Option<String>,

    /// When the issue was first seen in this project.
    #[serde(default)]
    pub first_found_at: Option<DateTime<Utc>>,

    /// When the revision was scanned.
    #[serde(default)]
    pub scanned_at: Option<DateTime<Utc>>,

    /// When the revision finished analysis.
    #[serde(default)]
    pub analyzed_at: Option<DateTime<Utc>>,
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

/// One decomposed CVSS metric, e.g. name "Attack Vector", value "Network".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueMetric {
    /// Metric name (e.g. "Attack Vector", "Privileges Required").
    pub name: String,

    /// Metric value (e.g. "Network", "None").
    #[serde(default)]
    pub value: Option<String>,
}

/// Issue category for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum IssueCategory {
    /// Security vulnerabilities.
    Vulnerability,
    /// License compliance issues.
    Licensing,
    /// Code quality concerns.
    Quality,
}

impl IssueCategory {
    /// Every category, in the order [`Issue::get`] probes them.
    pub const ALL: [IssueCategory; 3] = [
        IssueCategory::Vulnerability,
        IssueCategory::Licensing,
        IssueCategory::Quality,
    ];

    /// The lowercase string the API uses for this category.
    pub fn as_str(&self) -> &'static str {
        match self {
            IssueCategory::Vulnerability => "vulnerability",
            IssueCategory::Licensing => "licensing",
            IssueCategory::Quality => "quality",
        }
    }
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

    /// Fetch an issue by ID, discovering its category.
    ///
    /// The API requires a category and answers `404` when the ID exists under a
    /// different one, so this probes [`IssueCategory::ALL`] in order and returns
    /// the first hit — up to three requests. Prefer
    /// [`Issue::get_with_category`] when the category is already known.
    ///
    /// Only `404` advances to the next category; any other failure (auth, rate
    /// limit, server error) is returned as-is rather than being reported as a
    /// missing issue.
    #[tracing::instrument(skip(client))]
    async fn get(client: &FossaClient, id: Self::Id) -> Result<Self> {
        for category in IssueCategory::ALL {
            match Issue::get_with_category(client, id, category).await {
                Err(FossaError::ApiError {
                    status_code: Some(404),
                    ..
                }) => continue,
                result => return result,
            }
        }

        Err(FossaError::NotFound {
            entity_type: "Issue",
            id: id.to_string(),
        })
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

/// Reason recorded when ignoring an issue.
///
/// Serialized as the exact strings the API stores; anything else is silently
/// dropped server-side (the reason lookup returns NULL), which is why this is
/// an enum rather than a free-form string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ValueEnum)]
pub enum IssueIgnoreReason {
    /// The vulnerability has been fixed.
    #[serde(rename = "Fixed")]
    Fixed,
    /// Still being investigated.
    #[serde(rename = "Under_investigation")]
    UnderInvestigation,
    /// The advisory data is incorrect.
    #[serde(rename = "incorrect_data_found")]
    IncorrectDataFound,
    /// The affected component is not present.
    #[serde(rename = "Component_not_present")]
    ComponentNotPresent,
    /// The vulnerable code is not present.
    #[serde(rename = "Vulnerable_code_not_present")]
    VulnerableCodeNotPresent,
    /// The vulnerable code is never executed.
    #[serde(rename = "Vulnerable_code_not_in_execute_path")]
    VulnerableCodeNotInExecutePath,
    /// The vulnerable code cannot be controlled by an adversary.
    #[serde(rename = "Vulnerable_code_cannot_be_controlled_by_adversary")]
    VulnerableCodeCannotBeControlledByAdversary,
    /// Inline mitigations already exist.
    #[serde(rename = "Inline_mitigations_already_exist")]
    InlineMitigationsAlreadyExist,
    /// Some other reason (use notes to explain).
    #[serde(rename = "other")]
    Other,
}

/// The status-changing action sent in the body of `PUT /v2/issues/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum IssueAction {
    /// Ignore the issue, optionally with a comment and a reason.
    Ignore {
        /// Free-text comment shown alongside the ignore in the FOSSA UI.
        #[serde(skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
        /// Structured reason for the ignore.
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<IssueIgnoreReason>,
    },
    /// Revert a previous ignore, returning the issue to active.
    Unignore,
}

/// Parameters for [`Issue::update`].
#[derive(Debug, Clone)]
pub struct IssueUpdateParams {
    /// The issue's category. The API requires it on every issue write.
    pub category: IssueCategory,
    /// The action to perform.
    pub action: IssueAction,
}

/// Response body of `PUT /v2/issues/`.
///
/// `count` is the number of issue-project rows changed; `issue_id` is only
/// present when exactly one row changed.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueActionResponse {
    /// Number of issue-project rows the action changed.
    pub count: u64,
    /// The affected issue ID, present only when `count == 1`.
    #[serde(default)]
    pub issue_id: Option<u64>,
}

#[async_trait]
impl Update for Issue {
    type Id = u64;
    type Params = IssueUpdateParams;

    /// Ignore or un-ignore an issue, then return its refreshed state.
    ///
    /// Sends `PUT /v2/issues/` targeting this single issue. The API applies
    /// actions only to rows matching a status filter, so ignore targets
    /// `active` rows and unignore targets `ignored` rows; a `count` of 0 means
    /// nothing matched (wrong category, no access, or the issue is already in
    /// the requested state) and is surfaced as an error rather than silently
    /// succeeding.
    ///
    /// Requires a full API token: push-only tokens cannot write issues.
    #[tracing::instrument(skip(client))]
    async fn update(client: &FossaClient, id: u64, params: IssueUpdateParams) -> Result<Self> {
        let status = match params.action {
            IssueAction::Ignore { .. } => "active",
            IssueAction::Unignore => "ignored",
        };
        let path = format!(
            "v2/issues/?category={}&status={status}&ids[]={id}",
            params.category.as_str()
        );

        let response = client.put(&path, &params.action).await?;
        let result: IssueActionResponse =
            response.json().await.map_err(FossaError::HttpError)?;

        if result.count == 0 {
            return Err(FossaError::ApiError {
                message: format!(
                    "no {status} {} issue matched ID {id}; it may not exist, be outside \
                     your token's access, or already be in the requested state",
                    params.category.as_str()
                ),
                status_code: None,
            });
        }

        Issue::get_with_category(client, id, params.category).await
    }
}
