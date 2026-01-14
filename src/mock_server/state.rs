//! Mock server state management.
//!
//! Provides the in-memory data store for the mock FOSSA API server.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{Dependency, Issue, Project, Revision};

/// Shared state for the mock server.
///
/// This struct holds all the mock data that the server will serve.
/// It's wrapped in `Arc<RwLock<_>>` for concurrent access.
#[derive(Debug, Default)]
pub struct MockState {
    /// Projects indexed by locator (e.g., "custom+1/test-project").
    pub projects: HashMap<String, Project>,

    /// Revisions indexed by locator (e.g., "custom+1/test$main").
    pub revisions: HashMap<String, Revision>,

    /// Dependencies indexed by revision locator.
    /// Each revision can have multiple dependencies.
    pub dependencies: HashMap<String, Vec<Dependency>>,

    /// Issues indexed by ID.
    pub issues: HashMap<u64, Issue>,

    /// Optional authentication token. If set, requests must include this token.
    pub required_token: Option<String>,
}

impl MockState {
    /// Create a new empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create state wrapped in Arc<RwLock> for sharing.
    pub fn shared(self) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(self))
    }

    /// Add a project to the state.
    pub fn with_project(mut self, project: Project) -> Self {
        self.projects.insert(project.id.clone(), project);
        self
    }

    /// Add a revision to the state.
    pub fn with_revision(mut self, revision: Revision) -> Self {
        self.revisions.insert(revision.locator.clone(), revision);
        self
    }

    /// Add dependencies for a revision.
    pub fn with_dependencies(mut self, revision_locator: &str, deps: Vec<Dependency>) -> Self {
        self.dependencies
            .insert(revision_locator.to_string(), deps);
        self
    }

    /// Add an issue to the state.
    pub fn with_issue(mut self, issue: Issue) -> Self {
        self.issues.insert(issue.id, issue);
        self
    }

    /// Set the required authentication token.
    pub fn with_required_token(mut self, token: &str) -> Self {
        self.required_token = Some(token.to_string());
        self
    }

    /// Get a project by locator.
    pub fn get_project(&self, locator: &str) -> Option<&Project> {
        self.projects.get(locator)
    }

    /// Get a revision by locator.
    pub fn get_revision(&self, locator: &str) -> Option<&Revision> {
        self.revisions.get(locator)
    }

    /// Get dependencies for a revision.
    pub fn get_dependencies(&self, revision_locator: &str) -> Option<&Vec<Dependency>> {
        self.dependencies.get(revision_locator)
    }

    /// Get an issue by ID.
    pub fn get_issue(&self, id: u64) -> Option<&Issue> {
        self.issues.get(&id)
    }

    /// List all projects, optionally filtered by title.
    pub fn list_projects(&self, title_filter: Option<&str>) -> Vec<&Project> {
        self.projects
            .values()
            .filter(|p| {
                title_filter
                    .map(|t| p.title.to_lowercase().contains(&t.to_lowercase()))
                    .unwrap_or(true)
            })
            .collect()
    }

    /// List revisions for a project.
    pub fn list_revisions_for_project(&self, project_locator: &str) -> Vec<&Revision> {
        self.revisions
            .values()
            .filter(|r| {
                // Revision locator format: "project_locator$branch"
                r.locator.starts_with(project_locator)
                    && r.locator
                        .get(project_locator.len()..)
                        .map(|s| s.starts_with('$'))
                        .unwrap_or(false)
            })
            .collect()
    }

    /// List all issues, optionally filtered by category.
    pub fn list_issues(&self, category: Option<&str>) -> Vec<&Issue> {
        self.issues
            .values()
            .filter(|i| {
                category
                    .map(|c| i.issue_type.eq_ignore_ascii_case(c))
                    .unwrap_or(true)
            })
            .collect()
    }

    /// Update a project and return the updated version.
    pub fn update_project(
        &mut self,
        locator: &str,
        title: Option<String>,
        url: Option<String>,
        public: Option<bool>,
    ) -> Option<&Project> {
        if let Some(project) = self.projects.get_mut(locator) {
            if let Some(t) = title {
                project.title = t;
            }
            if let Some(u) = url {
                project.url = Some(u);
            }
            if let Some(p) = public {
                project.public = p;
            }
            return self.projects.get(locator);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_project(id: &str, title: &str) -> Project {
        Project {
            id: id.to_string(),
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

    #[test]
    fn test_state_add_and_get_project() {
        let state = MockState::new().with_project(sample_project("custom+1/test", "Test Project"));

        let project = state.get_project("custom+1/test");
        assert!(project.is_some());
        assert_eq!(project.unwrap().title, "Test Project");
    }

    #[test]
    fn test_state_list_projects_with_filter() {
        let state = MockState::new()
            .with_project(sample_project("custom+1/alpha", "Alpha Project"))
            .with_project(sample_project("custom+1/beta", "Beta Project"))
            .with_project(sample_project("custom+1/gamma", "Gamma Test"));

        let all = state.list_projects(None);
        assert_eq!(all.len(), 3);

        let filtered = state.list_projects(Some("project"));
        assert_eq!(filtered.len(), 2);

        let exact = state.list_projects(Some("gamma"));
        assert_eq!(exact.len(), 1);
    }

    #[test]
    fn test_state_update_project() {
        let mut state =
            MockState::new().with_project(sample_project("custom+1/test", "Original Title"));

        let updated = state.update_project(
            "custom+1/test",
            Some("New Title".to_string()),
            Some("https://example.com".to_string()),
            Some(true),
        );

        assert!(updated.is_some());
        let project = updated.unwrap();
        assert_eq!(project.title, "New Title");
        assert_eq!(project.url.as_deref(), Some("https://example.com"));
        assert!(project.public);
    }
}
