//! Snippet model and trait implementations.
//!
//! Snippet scanning identifies where a project's first-party source code matches
//! third-party open-source code (a legal/license-compliance risk). Snippets are a
//! child of a [`Revision`](crate::Revision): the revision locator is part of the
//! URL path, so listing uses a `(revision_locator, filters)` query tuple — the same
//! shape as [`Dependency`](crate::Dependency).
//!
//! The high-value endpoint is the *match details* drill-in
//! ([`get_snippet_match`]): it returns the actual lines (with line numbers) in the
//! user's **first-party file** alongside the reference open-source code, which is
//! what lets a remediation team jump straight to the matched code in their own repo.

#[cfg(test)]
mod tests {
    use super::*;

    // Real responses captured from the live API (see tests/fixtures/snippets/SCHEMA.md).
    const SNIPPETS_JSON: &str = include_str!("../../tests/fixtures/snippets/snippets.json");
    const DETAILS_JSON: &str = include_str!("../../tests/fixtures/snippets/snippet-details.json");
    const PATHS_JSON: &str = include_str!("../../tests/fixtures/snippets/paths.json");
    const MATCH_SMALL_JSON: &str =
        include_str!("../../tests/fixtures/snippets/match-details-small.json");
    const MATCH_PARTIAL_JSON: &str =
        include_str!("../../tests/fixtures/snippets/match-details-partial.json");

    #[test]
    fn test_snippet_list_response_deserialize() {
        let resp: SnippetListResponse =
            serde_json::from_str(SNIPPETS_JSON).expect("list deserialize");
        assert_eq!(resp.results.len(), 3);
        assert_eq!(resp.total_count, Some(3));
        assert_eq!(resp.page_size, Some(10));

        let first = &resp.results[0];
        // id is a STRING, not a number.
        assert_eq!(first.id, "1295019");
        assert_eq!(first.package, "Alamofire");
        assert_eq!(first.version, "5.11.0");
        assert_eq!(first.purl, "pkg:cocoapods/Alamofire@5.11.0");
        assert!(matches!(first.kind, SnippetKind::File));
        assert_eq!(first.match_count, 1);
        assert_eq!(first.license_ids(), vec!["MIT"]);
        // First result is rejected in this fixture.
        assert!(first.rejection_details.is_some());
        // Extra fields confirmed live.
        assert!(!first.is_vendored);
        assert!(!first.is_converted);
        assert_eq!(
            first.home_url.as_deref(),
            Some("https://github.com/alamofire/alamofire")
        );

        // Second result is vendored/converted and not rejected.
        let second = &resp.results[1];
        assert!(second.is_vendored);
        assert!(second.is_converted);
        assert!(second.rejection_details.is_none());
    }

    #[test]
    fn test_snippet_issue_counts_deserialize() {
        let resp: SnippetListResponse = serde_json::from_str(SNIPPETS_JSON).unwrap();
        let counts = &resp.results[0].issue_counts;
        assert_eq!(counts.licensing.flagged, 0);
        assert_eq!(counts.security.critical, 0);
    }

    #[test]
    fn test_snippet_details_deserialize() {
        let resp: SnippetDetailsResponse =
            serde_json::from_str(DETAILS_JSON).expect("details deserialize");
        let snippet = resp.snippet;
        assert_eq!(snippet.id, "1295019");

        // Details payload populates matches + otherVersions (absent from the list payload).
        assert_eq!(snippet.matches.len(), 1);
        assert_eq!(snippet.matches[0].path, "/Sources/Networking/Session.swift");
        assert_eq!(snippet.matches[0].match_percentage, 1.0);

        assert_eq!(snippet.other_versions.len(), 2);
        assert_eq!(snippet.other_versions[0].version, "5.9.1");
        assert_eq!(snippet.other_versions[0].match_count, 1);
    }

    #[test]
    fn test_snippet_list_payload_has_no_matches() {
        // The lighter list payload omits matches/otherVersions; #[serde(default)] must cover it.
        let resp: SnippetListResponse = serde_json::from_str(SNIPPETS_JSON).unwrap();
        assert!(resp.results[0].matches.is_empty());
        assert!(resp.results[0].other_versions.is_empty());
    }

    #[test]
    fn test_snippet_paths_deserialize() {
        let resp: SnippetPathsResponse =
            serde_json::from_str(PATHS_JSON).expect("paths deserialize");
        assert_eq!(resp.paths.len(), 1);
        let p = &resp.paths[0];
        assert_eq!(p.path_type, "directory");
        assert_eq!(p.name, "Sources");
        assert_eq!(p.path, "/Sources");
        assert_eq!(p.count, 3);
    }

    #[test]
    fn test_match_details_deserialize() {
        let resp: SnippetMatchDetailsResponse =
            serde_json::from_str(MATCH_SMALL_JSON).expect("match details deserialize");
        let md = resp.match_details;
        assert_eq!(md.path, "/Sources/Networking/Session.swift");
        assert_eq!(md.reference_code.len(), 6);
        assert_eq!(md.detected_code.len(), 6);

        let line = &md.detected_code[0];
        assert_eq!(line.line_number, 1);
        assert!(line.is_highlighted);
        assert_eq!(line.line, "//");
    }

    #[test]
    fn test_match_details_percentage_is_0_to_100_scale() {
        // The match-details endpoint reports matchPercentage on a 0-100 scale,
        // unlike the 0-1 scale used by the list/details endpoints.
        let resp: SnippetMatchDetailsResponse = serde_json::from_str(MATCH_SMALL_JSON).unwrap();
        assert_eq!(resp.match_details.match_percentage, 100.0);
    }

    #[test]
    fn test_detected_line_range_partial() {
        // A partial (kind:"snippet") match: only a sub-range of the file is highlighted.
        let resp: SnippetMatchDetailsResponse = serde_json::from_str(MATCH_PARTIAL_JSON).unwrap();
        let md = resp.match_details;
        assert_eq!(md.detected_line_range(), Some((41, 42)));
        assert_eq!(md.reference_line_range(), Some((13, 14)));
    }

    #[test]
    fn test_detected_line_range_none_when_no_highlight() {
        let md = SnippetMatchDetails {
            path: "x".into(),
            match_percentage: 0.5,
            reference_code: vec![],
            detected_code: vec![CodeLine {
                line: "a".into(),
                line_number: 7,
                is_highlighted: false,
            }],
        };
        assert_eq!(md.detected_line_range(), None);
    }

    #[test]
    fn test_line_range_ignores_trailing_blank_highlight() {
        let md = SnippetMatchDetails {
            path: "x".into(),
            match_percentage: 100.0,
            reference_code: vec![],
            detected_code: vec![
                CodeLine {
                    line: "fn a() {}".into(),
                    line_number: 10,
                    is_highlighted: true,
                },
                CodeLine {
                    line: String::new(),
                    line_number: 11,
                    is_highlighted: true,
                },
            ],
        };
        assert_eq!(md.detected_line_range(), Some((10, 10)));
    }

    #[test]
    fn test_snippet_kind_unknown_is_lenient() {
        let s: Snippet = serde_json::from_value(serde_json::json!({
            "id": "1", "packageId": "1", "purl": "p", "locator": "l",
            "package": "pkg", "version": "1.0", "kind": "somethingNew"
        }))
        .expect("unknown kind should deserialize");
        assert!(matches!(s.kind, SnippetKind::Unknown));
    }

    #[test]
    fn test_snippet_minimal_deserialize() {
        // Only the core identity fields are guaranteed; everything else must default.
        let s: Snippet = serde_json::from_value(serde_json::json!({
            "id": "1", "packageId": "1", "purl": "p", "locator": "l",
            "package": "pkg", "version": "1.0", "kind": "file"
        }))
        .expect("minimal deserialize");
        assert_eq!(s.match_count, 0);
        assert!(s.licenses.is_empty());
        assert!(s.matches.is_empty());
        assert_eq!(s.highest_match_percentage, 0.0);
    }

    #[test]
    fn test_snippet_list_query_serializes_only_set_fields() {
        // Unset fields must be skipped (the API rejects empty filter values).
        let q = SnippetListQuery {
            path: Some("/Sources".into()),
            search: None,
            sort: None,
        };
        let value = serde_json::to_value(&q).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.len(), 1);
        assert_eq!(obj.get("path").unwrap(), "/Sources");
    }

    #[test]
    fn test_with_default_path_fills_root() {
        assert_eq!(
            SnippetListQuery::default().with_default_path().path.as_deref(),
            Some("/")
        );
        assert_eq!(
            SnippetListQuery {
                path: Some("/Sources".into()),
                ..Default::default()
            }
            .with_default_path()
            .path
            .as_deref(),
            Some("/Sources")
        );
    }
}

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::client::FossaClient;
use crate::error::{FossaError, Result};
use crate::pagination::Page;
use crate::traits::List;

/// Maximum page size accepted by the snippets endpoint (`pageSize`, 1-50).
const SNIPPET_MAX_PAGE_SIZE: u32 = 50;

/// The kind of a snippet match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnippetKind {
    /// A whole-file match.
    File,
    /// A partial (sub-file) snippet match.
    Snippet,
    /// An unrecognized kind (forward-compatible).
    #[serde(other)]
    Unknown,
}

/// A snippet: a third-party open-source package matched against first-party code.
///
/// Returned by the snippet listing (`GET /revisions/{locator}/snippets`) and the
/// details endpoint (`GET /revisions/{locator}/snippets/{id}`). The `matches` and
/// `other_versions` fields are only populated by the details endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    /// Snippet identifier (a numeric string, e.g. "1295019").
    pub id: String,

    /// The matched package's identifier.
    pub package_id: String,

    /// Package URL (e.g. "pkg:cocoapods/Alamofire@5.11.0").
    pub purl: String,

    /// The matched package locator (e.g. "pod+Alamofire$5.11.0").
    pub locator: String,

    /// The matched package name.
    pub package: String,

    /// The matched package version.
    pub version: String,

    /// Whether this is a whole-file or partial snippet match.
    pub kind: SnippetKind,

    /// Highest match percentage across this snippet's matches (0.0-1.0).
    #[serde(default)]
    pub highest_match_percentage: f64,

    /// Number of files this snippet matched.
    #[serde(default)]
    pub match_count: u32,

    /// Licenses associated with the matched package.
    #[serde(default)]
    pub licenses: Vec<SnippetLicense>,

    /// Licensing and security issue counts for the matched package.
    #[serde(default)]
    pub issue_counts: SnippetIssueCounts,

    /// Rejection status, if this snippet has been rejected.
    #[serde(default)]
    pub rejection_details: Option<SnippetRejection>,

    /// Package labels.
    #[serde(default)]
    pub labels: Vec<SnippetLabel>,

    /// Package homepage URL.
    #[serde(default)]
    pub home_url: Option<String>,

    /// Package source code URL.
    #[serde(default)]
    pub code_url: Option<String>,

    /// Package release date.
    #[serde(default)]
    pub release_date: Option<DateTime<Utc>>,

    /// Whether the matched code is vendored into the project.
    #[serde(default)]
    pub is_vendored: bool,

    /// Whether the matched code was converted.
    #[serde(default)]
    pub is_converted: bool,

    /// Per-file matches (populated only by the details endpoint).
    #[serde(default)]
    pub matches: Vec<SnippetMatch>,

    /// Other versions of the matched package that also match (details endpoint only).
    #[serde(default)]
    pub other_versions: Vec<SnippetOtherVersion>,
}

impl Snippet {
    /// License identifiers (signatures) for the matched package.
    pub fn license_ids(&self) -> Vec<String> {
        self.licenses.iter().map(|l| l.signature.clone()).collect()
    }

    /// Whether this snippet has been rejected.
    pub fn is_rejected(&self) -> bool {
        self.rejection_details.is_some()
    }
}

/// A license associated with a matched snippet package.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetLicense {
    /// The license identifier/signature (e.g. "MIT").
    pub signature: String,

    /// How the license was determined (e.g. "declared", "discovered").
    #[serde(rename = "type", default)]
    pub license_type: Option<String>,

    /// Policy status (e.g. "approved", "denied", "flagged", "unknown").
    #[serde(default)]
    pub status: Option<String>,

    /// Associated issue identifier, if any.
    #[serde(default)]
    pub issue_id: Option<serde_json::Value>,
}

/// Licensing and security issue counts for a matched package.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnippetIssueCounts {
    /// Licensing issue counts.
    #[serde(default)]
    pub licensing: SnippetLicensingCounts,

    /// Security issue counts.
    #[serde(default)]
    pub security: SnippetSecurityCounts,
}

/// Licensing issue counts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnippetLicensingCounts {
    /// Number of denied licensing issues.
    #[serde(default)]
    pub denied: u32,
    /// Number of flagged licensing issues.
    #[serde(default)]
    pub flagged: u32,
    /// Number of unknown licensing issues.
    #[serde(default)]
    pub unknown: u32,
}

/// Security issue counts by severity.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnippetSecurityCounts {
    /// Critical-severity issues.
    #[serde(default)]
    pub critical: u32,
    /// High-severity issues.
    #[serde(default)]
    pub high: u32,
    /// Medium-severity issues.
    #[serde(default)]
    pub medium: u32,
    /// Low-severity issues.
    #[serde(default)]
    pub low: u32,
    /// Issues of unknown severity.
    #[serde(default)]
    pub unknown: u32,
}

/// Rejection details for a snippet or match.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetRejection {
    /// When the snippet was rejected.
    #[serde(default)]
    pub rejected_at: Option<DateTime<Utc>>,
    /// Who rejected the snippet.
    #[serde(default)]
    pub rejected_by: Option<String>,
}

/// A package label assignment. Fields are modeled leniently as the shape varies;
/// unknown fields are ignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetLabel {
    /// Label name.
    #[serde(default)]
    pub name: Option<String>,
    /// Label identifier.
    #[serde(default)]
    pub label_id: Option<serde_json::Value>,
    /// Label scope.
    #[serde(default)]
    pub scope: Option<String>,
}

/// Another version of a matched package that also matched (details endpoint).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetOtherVersion {
    /// The other version string.
    pub version: String,
    /// Number of matches for that version.
    #[serde(default)]
    pub match_count: u32,
}

/// A single first-party file where a snippet matched (from the details endpoint).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetMatch {
    /// The first-party file path.
    pub path: String,
    /// Match percentage for this file (0.0-1.0).
    #[serde(default)]
    pub match_percentage: f64,
    /// Rejection status for this match, if rejected.
    #[serde(default)]
    pub rejection_details: Option<SnippetRejection>,
}

/// An entry from the snippet path tree (`GET /revisions/{locator}/snippets/paths`).
///
/// The tree is hierarchical: querying `path=/` yields directories with a `count`;
/// drill in by querying a directory's `path` to reach files.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetPath {
    /// Either "directory" or "file".
    #[serde(rename = "type")]
    pub path_type: String,
    /// The entry name (file or directory name).
    pub name: String,
    /// The full path of the entry.
    pub path: String,
    /// Number of snippets at or under this entry.
    #[serde(default)]
    pub count: u32,
}

impl SnippetPath {
    /// Whether this entry is a file (vs a directory).
    pub fn is_file(&self) -> bool {
        self.path_type == "file"
    }
}

/// A single line of code in a match comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeLine {
    /// The line text.
    pub line: String,
    /// The 1-indexed line number in its file.
    pub line_number: u32,
    /// Whether this line is part of the highlighted match.
    #[serde(default)]
    pub is_highlighted: bool,
}

/// The drill-in payload for a single snippet match
/// (`GET /revisions/{locator}/snippets/{id}/matches/{path}`).
///
/// `detected_code` is the user's **first-party** code (with line numbers);
/// `reference_code` is the matched **open-source** code. This is what lets a team
/// locate the matched code in their own repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetMatchDetails {
    /// The first-party file path.
    pub path: String,
    /// Overall match percentage on a 0-100 scale.
    ///
    /// NOTE: this endpoint reports the percentage as 0-100 (e.g. `100`), unlike
    /// [`Snippet::highest_match_percentage`] and [`SnippetMatch::match_percentage`]
    /// which use a 0-1 scale. The value is kept as the API returns it.
    #[serde(default)]
    pub match_percentage: f64,
    /// Reference (open-source) code lines.
    #[serde(default)]
    pub reference_code: Vec<CodeLine>,
    /// Detected (first-party) code lines.
    #[serde(default)]
    pub detected_code: Vec<CodeLine>,
}

impl SnippetMatchDetails {
    fn highlighted_range(lines: &[CodeLine]) -> Option<(u32, u32)> {
        let nums = lines
            .iter()
            .filter(|l| l.is_highlighted && !l.line.trim().is_empty())
            .map(|l| l.line_number)
            .collect::<Vec<_>>();
        match (nums.iter().min(), nums.iter().max()) {
            (Some(&lo), Some(&hi)) => Some((lo, hi)),
            _ => None,
        }
    }

    /// The (start, end) first-party line range of the highlighted match, if any.
    pub fn detected_line_range(&self) -> Option<(u32, u32)> {
        Self::highlighted_range(&self.detected_code)
    }

    /// The (start, end) reference (open-source) line range of the highlighted match.
    pub fn reference_line_range(&self) -> Option<(u32, u32)> {
        Self::highlighted_range(&self.reference_code)
    }
}

/// A flattened remediation row: one matched first-party file location.
///
/// Produced by [`get_snippet_locations`]; this is the "where in my repo is this?"
/// map. The line range is only populated when requested (it requires a drill-in
/// call per match).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetLocation {
    /// The first-party file path where the match was found.
    pub path: String,
    /// The snippet identifier (use with [`get_snippet_match`] to drill in).
    pub snippet_id: String,
    /// The matched package name.
    pub package: String,
    /// The matched package version.
    pub version: String,
    /// The matched package URL.
    pub purl: String,
    /// Match percentage for this file (0.0-1.0).
    pub match_percentage: f64,
    /// License identifiers of the matched package.
    pub licenses: Vec<String>,
    /// First-party start line (only when computed via `with_lines`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_start: Option<u32>,
    /// First-party end line (only when computed via `with_lines`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_end: Option<u32>,
}

/// Query parameters for listing snippets / snippet paths.
///
/// `path` is required by the API; the convenience functions default it to `/` when
/// unset. Array filters (ids, packageIds, rejectionStatus, packageLabels) are not
/// modeled in v1 because the query is serialized via `serde_urlencoded`, which does
/// not support sequence fields.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SnippetListQuery {
    /// Filter by file/directory path (required by the API; defaults to `/`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Search term for filtering by package name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    /// Sort order (package_asc, package_desc, matchCount_asc, matchCount_desc).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
}

impl SnippetListQuery {
    /// Return a copy with `path` defaulted to `/` when unset (the API requires it).
    fn with_default_path(&self) -> Self {
        let mut q = self.clone();
        if q.path.is_none() {
            q.path = Some("/".to_string());
        }
        q
    }
}

/// Query type for snippet listing (includes the revision locator).
pub type SnippetQuery = (String, SnippetListQuery);

/// API response wrapper for listing snippets.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnippetListResponse {
    #[serde(default)]
    results: Vec<Snippet>,
    #[serde(default)]
    total_count: Option<u64>,
    #[serde(default)]
    #[allow(dead_code)]
    page: Option<u32>,
    #[serde(default)]
    #[allow(dead_code)]
    page_size: Option<u32>,
}

/// API response wrapper for snippet details.
#[derive(Debug, Deserialize)]
struct SnippetDetailsResponse {
    snippet: Snippet,
}

/// API response wrapper for the snippet path tree.
#[derive(Debug, Deserialize)]
struct SnippetPathsResponse {
    #[serde(default)]
    paths: Vec<SnippetPath>,
}

/// API response wrapper for match details.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnippetMatchDetailsResponse {
    match_details: SnippetMatchDetails,
}

#[async_trait]
impl List for Snippet {
    type Query = SnippetQuery; // (revision_locator, filters)

    #[tracing::instrument(skip(client))]
    async fn list_page(
        client: &FossaClient,
        query: &Self::Query,
        page: u32,
        count: u32,
    ) -> Result<Page<Self>> {
        let (revision_locator, filters) = query;
        let encoded_locator = urlencoding::encode(revision_locator);
        let path = format!("revisions/{encoded_locator}/snippets");

        // The API caps pageSize at 50.
        let page_size = count.clamp(1, SNIPPET_MAX_PAGE_SIZE);
        let effective = filters.with_default_path();

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct RequestParams<'a> {
            #[serde(flatten)]
            query: &'a SnippetListQuery,
            page: u32,
            page_size: u32,
        }

        let params = RequestParams {
            query: &effective,
            page,
            page_size,
        };

        let response = client.get_with_query(&path, &params).await?;
        let data: SnippetListResponse = response.json().await.map_err(FossaError::HttpError)?;

        // Report has_more using the actual page size used, so list_all paginates correctly.
        Ok(Page::new(data.results, page, page_size, data.total_count))
    }

    /// Override the default `list_all`: it paginates at `DEFAULT_PAGE_SIZE` (100) and
    /// stops on the first short page, which would be wrong here since the API caps
    /// pages at 50. Page at 50 and rely on `has_more` (derived from `totalCount`).
    async fn list_all(client: &FossaClient, query: &Self::Query) -> Result<Vec<Self>> {
        let mut all = Vec::new();
        let mut page = 1u32;
        loop {
            let result = Self::list_page(client, query, page, SNIPPET_MAX_PAGE_SIZE).await?;
            let count = result.items.len();
            all.extend(result.items);
            if !result.has_more || count == 0 {
                break;
            }
            page += 1;
            if page > 1000 {
                tracing::warn!("Reached snippet pagination limit, stopping");
                break;
            }
        }
        Ok(all)
    }
}

// -----------------------------------------------------------------------------
// Convenience functions
// -----------------------------------------------------------------------------

/// Fetch all snippets for a revision (all pages).
pub async fn get_snippets(
    client: &FossaClient,
    revision_locator: &str,
    query: SnippetListQuery,
) -> Result<Vec<Snippet>> {
    Snippet::list_all(client, &(revision_locator.to_string(), query)).await
}

/// Fetch a single page of snippets for a revision.
pub async fn get_snippets_page(
    client: &FossaClient,
    revision_locator: &str,
    query: SnippetListQuery,
    page: u32,
    count: u32,
) -> Result<Page<Snippet>> {
    Snippet::list_page(client, &(revision_locator.to_string(), query), page, count).await
}

/// Fetch the snippet path tree for a revision.
///
/// The tree is hierarchical — pass a directory `path` in the query to drill in.
/// When `path` is unset it defaults to `/` (the repository root).
pub async fn get_snippet_paths(
    client: &FossaClient,
    revision_locator: &str,
    query: SnippetListQuery,
) -> Result<Vec<SnippetPath>> {
    let encoded_locator = urlencoding::encode(revision_locator);
    let path = format!("revisions/{encoded_locator}/snippets/paths");
    let effective = query.with_default_path();
    let response = client.get_with_query(&path, &effective).await?;
    let data: SnippetPathsResponse = response.json().await.map_err(FossaError::HttpError)?;
    Ok(data.paths)
}

/// Fetch full details for a single snippet, including its per-file `matches`.
pub async fn get_snippet_details(
    client: &FossaClient,
    revision_locator: &str,
    snippet_id: &str,
) -> Result<Snippet> {
    let encoded_locator = urlencoding::encode(revision_locator);
    let encoded_id = urlencoding::encode(snippet_id);
    let path = format!("revisions/{encoded_locator}/snippets/{encoded_id}");
    let response = client.get(&path).await?;
    let data: SnippetDetailsResponse = response.json().await.map_err(FossaError::HttpError)?;
    Ok(data.snippet)
}

/// Fetch the match-details drill-in for a snippet at a specific first-party path.
///
/// Returns the first-party (`detected_code`) and open-source (`reference_code`)
/// lines with line numbers — the data needed to locate the match in the repo.
pub async fn get_snippet_match(
    client: &FossaClient,
    revision_locator: &str,
    snippet_id: &str,
    match_path: &str,
) -> Result<SnippetMatchDetails> {
    let encoded_locator = urlencoding::encode(revision_locator);
    let encoded_id = urlencoding::encode(snippet_id);
    let encoded_path = urlencoding::encode(match_path);
    let path =
        format!("revisions/{encoded_locator}/snippets/{encoded_id}/matches/{encoded_path}");
    let response = client.get(&path).await?;
    let data: SnippetMatchDetailsResponse =
        response.json().await.map_err(FossaError::HttpError)?;
    Ok(data.match_details)
}

/// Build a flat list of every snippet match location in a revision.
///
/// This is the aggregating "where in my repo is this?" report: it lists all
/// snippets, fetches each snippet's details to enumerate its matched files, and
/// emits one [`SnippetLocation`] per (snippet, file).
///
/// When `with_lines` is true, it additionally drills into each match to compute the
/// highlighted first-party line range — at the cost of one extra (potentially large)
/// request per match. Drill-in failures degrade gracefully to an empty line range.
///
/// Cost: ~1 list + N details calls (one per snippet), plus M match-details calls
/// when `with_lines` is set. Scope the work with the query's `path`/`search` filters.
pub async fn get_snippet_locations(
    client: &FossaClient,
    revision_locator: &str,
    query: SnippetListQuery,
    with_lines: bool,
) -> Result<Vec<SnippetLocation>> {
    let snippets = get_snippets(client, revision_locator, query).await?;

    let mut locations = Vec::new();
    for snippet in &snippets {
        let details = get_snippet_details(client, revision_locator, &snippet.id).await?;
        let licenses = details.license_ids();
        for m in &details.matches {
            let (line_start, line_end) = if with_lines {
                match get_snippet_match(client, revision_locator, &snippet.id, &m.path).await {
                    Ok(md) => match md.detected_line_range() {
                        Some((lo, hi)) => (Some(lo), Some(hi)),
                        None => (None, None),
                    },
                    Err(e) => {
                        tracing::warn!(
                            "Failed to fetch match details for {} at {}: {e}",
                            snippet.id,
                            m.path
                        );
                        (None, None)
                    }
                }
            } else {
                (None, None)
            };

            locations.push(SnippetLocation {
                path: m.path.clone(),
                snippet_id: snippet.id.clone(),
                package: details.package.clone(),
                version: details.version.clone(),
                purl: details.purl.clone(),
                match_percentage: m.match_percentage,
                licenses: licenses.clone(),
                line_start,
                line_end,
            });
        }
    }

    Ok(locations)
}
