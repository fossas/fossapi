//! Test data fixtures for the mock server.
//!
//! Provides factory functions for creating realistic test data.

use crate::{
    Dependency, Issue, IssueDepths, IssueSource, IssueStatuses, LatestRevision, Project,
    ProjectIssues, Revision,
};

/// Collection of fixture factories for test data.
pub struct Fixtures;

impl Fixtures {
    // =========================================================================
    // Project Fixtures
    // =========================================================================

    /// Create a minimal project with required fields only.
    pub fn minimal_project(locator: &str, title: &str) -> Project {
        Project {
            id: locator.to_string(),
            title: title.to_string(),
            branch: None,
            version: None,
            project_type: None,
            url: None,
            public: false,
            scanned: None,
            last_analyzed: None,
            issues: None,
            labels: vec![],
            teams: vec![],
            latest_revision: None,
            latest_build_status: None,
        }
    }

    /// Create a project with issue counts.
    pub fn project_with_issues(
        locator: &str,
        title: &str,
        vuln: u32,
        licensing: u32,
        quality: u32,
    ) -> Project {
        let mut project = Self::minimal_project(locator, title);
        project.issues = Some(ProjectIssues {
            total: vuln + licensing + quality,
            licensing,
            security: vuln,
            quality,
        });
        project
    }

    /// Create an analyzed project with a latest revision.
    pub fn analyzed_project(locator: &str, title: &str, branch: &str) -> Project {
        let mut project = Self::minimal_project(locator, title);
        project.branch = Some(branch.to_string());
        project.latest_revision = Some(LatestRevision {
            locator: format!("{}${}", locator, branch),
            message: Some("Latest analysis".to_string()),
        });
        project.latest_build_status = Some("SUCCEEDED".to_string());
        project
    }

    // =========================================================================
    // Revision Fixtures
    // =========================================================================

    /// Create a minimal revision.
    pub fn minimal_revision(locator: &str) -> Revision {
        Revision {
            locator: locator.to_string(),
            project_id: None,
            resolved: true,
            source_type: None,
            source: None,
            message: None,
            error: None,
            created_at: None,
            updated_at: None,
            revision_timestamp: None,
            latest_revision_scan_id: None,
            latest_hubble_analysis_id: None,
            author: None,
            link: None,
            url: None,
            unresolved_issue_count: None,
            loc: None,
        }
    }

    /// Create a resolved revision with common fields.
    pub fn resolved_revision(locator: &str, source_type: &str) -> Revision {
        let mut rev = Self::minimal_revision(locator);
        rev.resolved = true;
        rev.source_type = Some(source_type.to_string());
        rev.source = Some("cli".to_string());
        rev
    }

    // =========================================================================
    // Dependency Fixtures
    // =========================================================================

    /// Create a minimal dependency.
    pub fn minimal_dependency(locator: &str, depth: u32) -> Dependency {
        Dependency {
            locator: locator.to_string(),
            title: None,
            depth,
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

    /// Create an npm dependency.
    pub fn npm_dependency(name: &str, version: &str, depth: u32) -> Dependency {
        let mut dep = Self::minimal_dependency(&format!("npm+{}${}", name, version), depth);
        dep.title = Some(name.to_string());
        dep.version_field = Some(version.to_string());
        dep
    }

    // =========================================================================
    // Issue Fixtures
    // =========================================================================

    /// Create a vulnerability issue.
    pub fn vulnerability_issue(
        id: u64,
        cve: &str,
        severity: &str,
        package_locator: &str,
    ) -> Issue {
        Issue {
            id,
            issue_type: "vulnerability".to_string(),
            source: IssueSource {
                id: package_locator.to_string(),
                name: None,
                url: None,
                version: None,
                package_manager: None,
            },
            depths: IssueDepths { direct: 1, deep: 0 },
            statuses: IssueStatuses {
                active: 1,
                ignored: 0,
            },
            projects: vec![],
            created_at: None,
            cve: Some(cve.to_string()),
            cvss: Some(7.5),
            cvss_vector: None,
            severity: Some(severity.to_string()),
            details: Some(format!("Vulnerability {} in package", cve)),
            remediation: None,
            cwes: vec![],
            published: None,
            exploitability: None,
            epss: None,
            vuln_id: Some(format!("{}_{}", cve, package_locator)),
            title: Some(format!("{} Vulnerability", cve)),
            license: None,
            quality_rule: None,
        }
    }

    /// Create a licensing issue.
    pub fn licensing_issue(id: u64, license: &str, package_locator: &str) -> Issue {
        Issue {
            id,
            issue_type: "licensing".to_string(),
            source: IssueSource {
                id: package_locator.to_string(),
                name: None,
                url: None,
                version: None,
                package_manager: None,
            },
            depths: IssueDepths { direct: 0, deep: 1 },
            statuses: IssueStatuses {
                active: 1,
                ignored: 0,
            },
            projects: vec![],
            created_at: None,
            cve: None,
            cvss: None,
            cvss_vector: None,
            severity: None,
            details: None,
            remediation: None,
            cwes: vec![],
            published: None,
            exploitability: None,
            epss: None,
            vuln_id: None,
            title: None,
            license: Some(license.to_string()),
            quality_rule: None,
        }
    }

    // =========================================================================
    // Scenario Builders
    // =========================================================================

    /// Create a default set of test data for common scenarios.
    pub fn default_scenario() -> DefaultScenario {
        DefaultScenario::new()
    }
}

/// A complete test scenario with related entities.
pub struct DefaultScenario {
    pub projects: Vec<Project>,
    pub revisions: Vec<Revision>,
    pub dependencies: Vec<(String, Vec<Dependency>)>,
    pub issues: Vec<Issue>,
}

impl DefaultScenario {
    fn new() -> Self {
        let project_locator = "custom+1/test-project";
        let revision_locator = format!("{}$main", project_locator);

        let projects = vec![Fixtures::analyzed_project(
            project_locator,
            "Test Project",
            "main",
        )];

        let revisions = vec![Fixtures::resolved_revision(&revision_locator, "npm")];

        let dependencies = vec![(
            revision_locator.clone(),
            vec![
                Fixtures::npm_dependency("lodash", "4.17.21", 1),
                Fixtures::npm_dependency("express", "4.18.0", 1),
                Fixtures::npm_dependency("accepts", "1.3.8", 2),
            ],
        )];

        let issues = vec![
            Fixtures::vulnerability_issue(
                1,
                "CVE-2024-0001",
                "high",
                "npm+lodash$4.17.21",
            ),
            Fixtures::licensing_issue(2, "GPL-3.0", "npm+gpl-package$1.0.0"),
        ];

        Self {
            projects,
            revisions,
            dependencies,
            issues,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_project() {
        let project = Fixtures::minimal_project("custom+1/test", "Test");
        assert_eq!(project.id, "custom+1/test");
        assert_eq!(project.title, "Test");
        assert!(!project.public);
    }

    #[test]
    fn test_project_with_issues() {
        let project = Fixtures::project_with_issues("custom+1/test", "Test", 5, 3, 2);
        let issues = project.issues.unwrap();
        assert_eq!(issues.total, 10);
        assert_eq!(issues.security, 5);
        assert_eq!(issues.licensing, 3);
        assert_eq!(issues.quality, 2);
    }

    #[test]
    fn test_vulnerability_issue() {
        let issue =
            Fixtures::vulnerability_issue(42, "CVE-2024-1234", "critical", "npm+lodash$4.17.0");
        assert_eq!(issue.id, 42);
        assert_eq!(issue.issue_type, "vulnerability");
        assert_eq!(issue.cve.as_deref(), Some("CVE-2024-1234"));
        assert_eq!(issue.severity.as_deref(), Some("critical"));
    }

    #[test]
    fn test_default_scenario() {
        let scenario = Fixtures::default_scenario();
        assert!(!scenario.projects.is_empty());
        assert!(!scenario.revisions.is_empty());
        assert!(!scenario.dependencies.is_empty());
        assert!(!scenario.issues.is_empty());
    }
}
