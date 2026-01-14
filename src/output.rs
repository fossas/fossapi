//! Output formatting for CLI display.
//!
//! Provides the [`PrettyPrint`] trait for human-readable output
//! as an alternative to JSON serialization.

use crate::{Issue, Project, Revision};

/// Trait for human-readable key-value output.
///
/// Implemented by entity types to provide formatted output
/// suitable for terminal display when `--json` is not specified.
pub trait PrettyPrint {
    /// Returns a formatted string for terminal display.
    fn pretty_print(&self) -> String;
}

impl PrettyPrint for Project {
    fn pretty_print(&self) -> String {
        let locator = self.locator();
        let divider = "─".repeat(locator.len().max(30));

        let mut lines = vec![
            format!("Project: {}", locator),
            divider,
            format!("Title:          {}", self.title),
        ];

        if let Some(ref issues) = self.issues {
            lines.push(format!(
                "Issues:         {} ({} security, {} licensing, {} quality)",
                issues.total, issues.security, issues.licensing, issues.quality
            ));
        }

        if let Some(ref latest) = self.latest_revision {
            lines.push(format!("Latest Rev:     {}", latest.locator));
        }

        if let Some(ref scanned) = self.scanned {
            lines.push(format!("Scanned:        {}", scanned.format("%Y-%m-%d %H:%M:%S UTC")));
        }

        if self.public {
            lines.push("Visibility:     public".to_string());
        }

        lines.join("\n")
    }
}

impl PrettyPrint for Revision {
    fn pretty_print(&self) -> String {
        let divider = "─".repeat(self.locator.len().max(30));

        let mut lines = vec![
            format!("Revision: {}", self.locator),
            divider,
            format!("Resolved:       {}", if self.resolved { "yes" } else { "no" }),
        ];

        if let Some(ref source) = self.source {
            lines.push(format!("Source:         {}", source));
        }

        if let Some(ref source_type) = self.source_type {
            lines.push(format!("Source Type:    {}", source_type));
        }

        if let Some(ref created) = self.created_at {
            lines.push(format!("Created:        {}", created.format("%Y-%m-%d %H:%M:%S UTC")));
        }

        if let Some(count) = self.unresolved_issue_count {
            lines.push(format!("Unresolved:     {} issues", count));
        }

        lines.join("\n")
    }
}

impl PrettyPrint for Issue {
    fn pretty_print(&self) -> String {
        let header = format!("Issue #{}", self.id);
        let divider = "─".repeat(header.len().max(30));

        let mut lines = vec![
            header,
            divider,
            format!("Type:           {}", self.issue_type),
        ];

        if let Some(ref severity) = self.severity {
            lines.push(format!("Severity:       {}", severity));
        }

        if let Some(ref cve) = self.cve {
            lines.push(format!("CVE:            {}", cve));
        }

        // Source package info
        if let Some(ref name) = self.source.name {
            lines.push(format!("Source:         {}", name));
        } else {
            lines.push(format!("Source:         {}", self.source.id));
        }

        // Status counts
        lines.push(format!(
            "Status:         {} active, {} ignored",
            self.statuses.active, self.statuses.ignored
        ));

        // Depth info
        lines.push(format!(
            "Depths:         {} direct, {} transitive",
            self.depths.direct, self.depths.deep
        ));

        if let Some(ref license) = self.license {
            lines.push(format!("License:        {}", license));
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_pretty_print_format() {
        let project: Project = serde_json::from_value(serde_json::json!({
            "id": "custom+org/my-project",
            "title": "My Project",
            "public": false,
            "labels": [],
            "teams": []
        }))
        .unwrap();

        let output = project.pretty_print();
        assert!(output.starts_with("Project:"));
        assert!(output.contains("Title:"));
    }
}
