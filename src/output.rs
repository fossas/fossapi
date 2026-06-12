//! Output formatting for CLI display.
//!
//! Provides the [`PrettyPrint`] trait for human-readable output
//! as an alternative to JSON serialization.

use std::collections::BTreeSet;

use crate::{CodeLine, Issue, Project, Revision, Snippet, SnippetKind, SnippetMatchDetails};

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
            lines.push(format!("Source:         {source}"));
        }

        if let Some(ref source_type) = self.source_type {
            lines.push(format!("Source Type:    {source_type}"));
        }

        if let Some(ref created) = self.created_at {
            lines.push(format!("Created:        {}", created.format("%Y-%m-%d %H:%M:%S UTC")));
        }

        if let Some(count) = self.unresolved_issue_count {
            lines.push(format!("Unresolved:     {count} issues"));
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
            lines.push(format!("Severity:       {severity}"));
        }

        if let Some(ref cve) = self.cve {
            lines.push(format!("CVE:            {cve}"));
        }

        // Source package info
        if let Some(ref name) = self.source.name {
            lines.push(format!("Source:         {name}"));
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
            lines.push(format!("License:        {license}"));
        }

        lines.join("\n")
    }
}

fn snippet_kind_label(kind: SnippetKind) -> &'static str {
    match kind {
        SnippetKind::File => "file (whole-file match)",
        SnippetKind::Snippet => "snippet (partial match)",
        SnippetKind::Unknown => "unknown",
    }
}

impl PrettyPrint for Snippet {
    fn pretty_print(&self) -> String {
        let header = format!("Snippet #{}", self.id);
        let divider = "─".repeat(header.len().max(30));

        let mut lines = vec![
            header,
            divider,
            format!("Package:        {} {}", self.package, self.version),
            format!("PURL:           {}", self.purl),
            format!("Kind:           {}", snippet_kind_label(self.kind)),
            format!(
                "Match:          {:.0}% (top), {} file(s)",
                self.highest_match_percentage * 100.0,
                self.match_count
            ),
        ];

        let license_ids = self.license_ids();
        if !license_ids.is_empty() {
            lines.push(format!("Licenses:       {}", license_ids.join(", ")));
        }

        if self.is_rejected() {
            let by = self
                .rejection_details
                .as_ref()
                .and_then(|r| r.rejected_by.as_deref())
                .unwrap_or("unknown");
            lines.push(format!("Rejected:       yes (by {by})"));
        }

        if !self.matches.is_empty() {
            lines.push(String::new());
            lines.push("Matched files:".to_string());
            for m in &self.matches {
                lines.push(format!("  {:.0}%  {}", m.match_percentage * 100.0, m.path));
            }
        }

        lines.join("\n")
    }
}

impl PrettyPrint for SnippetMatchDetails {
    fn pretty_print(&self) -> String {
        let header = format!("Snippet match: {}", self.path);
        let divider = "─".repeat(header.len().max(30));

        let mut lines = vec![header, divider];
        lines.push(format!("Match:          {:.0}%", self.match_percentage));
        if let Some((lo, hi)) = self.detected_line_range() {
            lines.push(format!("Your code:      lines {lo}-{hi}"));
        }
        if let Some((lo, hi)) = self.reference_line_range() {
            lines.push(format!("Reference:      lines {lo}-{hi}"));
        }

        lines.push(String::new());
        lines.push("── Detected (your code) ──".to_string());
        lines.extend(render_code_block(&self.detected_code));
        lines.push(String::new());
        lines.push("── Reference (open source) ──".to_string());
        lines.extend(render_code_block(&self.reference_code));

        lines.join("\n")
    }
}

fn visible_indices(code: &[CodeLine], context: usize) -> BTreeSet<usize> {
    let highlighted = code
        .iter()
        .enumerate()
        .filter(|(_, l)| l.is_highlighted)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();

    if highlighted.is_empty() {
        return (0..code.len()).collect();
    }

    let mut shown = BTreeSet::new();
    for i in highlighted {
        let lo = i.saturating_sub(context);
        let hi = (i + context).min(code.len() - 1);
        shown.extend(lo..=hi);
    }
    shown
}

fn render_code_block(code: &[CodeLine]) -> Vec<String> {
    const CONTEXT: usize = 3;

    if code.is_empty() {
        return vec!["  (no code)".to_string()];
    }

    let mut out = Vec::new();
    let mut prev = None;
    for i in visible_indices(code, CONTEXT) {
        if prev.is_some_and(|p| i > p + 1) {
            out.push("        ⋮".to_string());
        }
        let l = &code[i];
        let gutter = if l.is_highlighted { '>' } else { ' ' };
        out.push(format!("{gutter} {:>5} │ {}", l.line_number, l.line));
        prev = Some(i);
    }
    out
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
