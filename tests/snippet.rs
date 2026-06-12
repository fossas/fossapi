//! Integration tests for the snippet convenience functions.
//!
//! Uses wiremock to mock the FOSSA API and exercises the public library surface
//! (get_snippets, get_snippet_paths, get_snippet_details, get_snippet_match,
//! get_snippet_locations) against the real response shapes captured from the API.

use fossapi::FossaClient;
use wiremock::matchers::{method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const REV: &str = "custom+1/repo$abc";
const ENC_REV: &str = "custom%2B1%2Frepo%24abc";

const SNIPPETS_JSON: &str = include_str!("fixtures/snippets/snippets.json");
const PATHS_JSON: &str = include_str!("fixtures/snippets/paths.json");
const DETAILS_JSON: &str = include_str!("fixtures/snippets/snippet-details.json");
const MATCH_SMALL_JSON: &str = include_str!("fixtures/snippets/match-details-small.json");

fn body(json: &str) -> serde_json::Value {
    serde_json::from_str(json).expect("fixture is valid JSON")
}

#[tokio::test]
async fn get_snippet_paths_returns_tree() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/revisions/{ENC_REV}/snippets/paths")))
        .and(query_param("path", "/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body(PATHS_JSON)))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("t", &mock_server.uri()).unwrap();
    let paths = fossapi::get_snippet_paths(&client, REV, Default::default())
        .await
        .expect("get_snippet_paths");

    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].path, "/Sources");
    assert!(!paths[0].is_file());
    assert_eq!(paths[0].count, 3);
}

#[tokio::test]
async fn get_snippets_returns_results() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/revisions/{ENC_REV}/snippets")))
        .respond_with(ResponseTemplate::new(200).set_body_json(body(SNIPPETS_JSON)))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("t", &mock_server.uri()).unwrap();
    let snippets = fossapi::get_snippets(&client, REV, Default::default())
        .await
        .expect("get_snippets");

    assert_eq!(snippets.len(), 3);
    assert_eq!(snippets[0].id, "1295019");
    assert_eq!(snippets[0].package, "Alamofire");
    assert_eq!(snippets[0].license_ids(), vec!["MIT"]);
}

#[tokio::test]
async fn get_snippet_details_populates_matches() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/revisions/{ENC_REV}/snippets/1295019")))
        .respond_with(ResponseTemplate::new(200).set_body_json(body(DETAILS_JSON)))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("t", &mock_server.uri()).unwrap();
    let snippet = fossapi::get_snippet_details(&client, REV, "1295019")
        .await
        .expect("get_snippet_details");

    assert_eq!(snippet.matches.len(), 1);
    assert_eq!(snippet.matches[0].path, "/Sources/Networking/Session.swift");
    assert_eq!(snippet.other_versions.len(), 2);
}

#[tokio::test]
async fn get_snippet_match_returns_code_with_line_numbers() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_regex(r"/snippets/1295019/matches/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body(MATCH_SMALL_JSON)))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("t", &mock_server.uri()).unwrap();
    let details =
        fossapi::get_snippet_match(&client, REV, "1295019", "/Sources/Networking/Session.swift")
            .await
            .expect("get_snippet_match");

    assert_eq!(details.detected_code.len(), 6);
    assert_eq!(details.detected_code[0].line_number, 1);
    assert!(details.detected_code[0].is_highlighted);
}

#[tokio::test]
async fn get_snippet_locations_flattens_matches() {
    let mock_server = MockServer::start().await;

    // List enumerates the snippets...
    Mock::given(method("GET"))
        .and(path(format!("/revisions/{ENC_REV}/snippets")))
        .respond_with(ResponseTemplate::new(200).set_body_json(body(SNIPPETS_JSON)))
        .mount(&mock_server)
        .await;

    // ...then each snippet's details provide its matched files.
    Mock::given(method("GET"))
        .and(path_regex(r"/snippets/[0-9]+$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body(DETAILS_JSON)))
        .mount(&mock_server)
        .await;

    let client = FossaClient::new("t", &mock_server.uri()).unwrap();
    let locations = fossapi::get_snippet_locations(&client, REV, Default::default(), false)
        .await
        .expect("get_snippet_locations");

    // Three snippets, each with one match in the (shared) details fixture.
    assert_eq!(locations.len(), 3);
    // snippet_id comes from the list; the ids are distinct.
    let ids: Vec<&str> = locations.iter().map(|l| l.snippet_id.as_str()).collect();
    assert_eq!(ids, vec!["1295019", "1295021", "1695309"]);
    // Without with_lines, no line range is resolved.
    assert!(locations[0].line_start.is_none());
    assert_eq!(locations[0].path, "/Sources/Networking/Session.swift");
}
