//! MCP Server handler for FOSSA API.

use rmcp::{
    handler::server::ServerHandler,
    model::{
        CallToolRequestParam, CallToolResult, Content, ErrorData as McpError, Implementation,
        ListToolsResult, PaginatedRequestParam, ServerCapabilities, ServerInfo, Tool,
        ToolsCapability,
    },
    service::RequestContext,
    RoleServer,
};
use schemars::JsonSchema;
use serde::Serialize;
use std::sync::Arc;

use crate::{
    ops::{
        run_get, run_ignore, run_list, run_unignore, run_update, GetCommand, IgnoreCommand,
        ListCommand, UnignoreCommand, UpdateCommand,
    },
    FossaClient, FossaError,
};

/// FOSSA MCP Server.
///
/// Implements the MCP ServerHandler trait, providing tools to interact
/// with the FOSSA API through the Model Context Protocol.
///
/// # Tools
///
/// The server exposes one tool per CLI verb — `get`, `list`, `update`,
/// `ignore`, and `unignore` — and each tool's input schema is generated from the same
/// [`crate::ops`] enum that the CLI parses into, so the two surfaces
/// always expose the same operations with the same parameters.
///
/// # Example
///
/// ```no_run
/// use fossapi::mcp::FossaServer;
///
/// # fn main() -> fossapi::Result<()> {
/// let server = FossaServer::from_env()?;
/// // Server can now be used with rmcp transport
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct FossaServer {
    client: Arc<FossaClient>,
}

impl FossaServer {
    /// Create a new FossaServer from environment variables.
    ///
    /// Uses `FOSSA_API_KEY` for authentication and optionally `FOSSA_API_URL`
    /// for the base URL.
    ///
    /// # Errors
    ///
    /// Returns an error if `FOSSA_API_KEY` is not set.
    pub fn from_env() -> crate::Result<Self> {
        let client = FossaClient::from_env()?;
        Ok(Self::new(client))
    }

    /// Create a new FossaServer with an existing client.
    pub fn new(client: FossaClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }

    /// The MCP tools this server exposes: one per verb, with input schemas
    /// generated from the shared [`crate::ops`] command enums.
    pub fn tools() -> Vec<Tool> {
        vec![
            Tool::new(
                "get",
                "Fetch a single FOSSA entity. entity values: project (by locator), \
                 revision (by locator), issue (by numeric id; category optional — when \
                 omitted, every category is probed, up to 3 requests), snippet (details \
                 including matched first-party files), snippet_match (side-by-side \
                 first-party detected_code vs open-source reference_code for one \
                 snippet at one path).",
                Self::schema::<GetCommand>(),
            ),
            Tool::new(
                "list",
                "List FOSSA entities. entity values: projects (paginated), issues \
                 (category required: vulnerability, licensing, quality), dependencies \
                 (revision locator required), revisions (project locator required), \
                 snippets (revision locator; one row per matched OSS package), \
                 snippet_locations (revision locator; one row per matched first-party \
                 file, optional with_lines resolves line ranges), snippet_paths \
                 (revision locator; the file/directory tree where snippets were \
                 detected).",
                Self::schema::<ListCommand>(),
            ),
            Tool::new(
                "update",
                "Update a FOSSA entity. entity values: project (title, description, \
                 url, public, policy_id, default_branch).",
                Self::schema::<UpdateCommand>(),
            ),
            Tool::new(
                "ignore",
                "Ignore a FOSSA issue, optionally recording notes (a free-text \
                 comment) and, for vulnerability issues only, a structured reason. \
                 entity values: issue (numeric id; category required). Refused if \
                 the issue is already fully ignored — unignore it first to change \
                 its notes or reason. Requires a full API token (push-only tokens \
                 cannot write issues).",
                Self::schema::<IgnoreCommand>(),
            ),
            Tool::new(
                "unignore",
                "Revert a previous ignore, returning a FOSSA issue to active. \
                 entity values: issue (numeric id; category required). Refused if \
                 nothing on the issue is ignored. Requires a full API token.",
                Self::schema::<UnignoreCommand>(),
            ),
        ]
    }

    /// Generate JSON Schema for a type.
    fn schema<T: JsonSchema>() -> Arc<serde_json::Map<String, serde_json::Value>> {
        let schema = schemars::schema_for!(T);
        let value = serde_json::to_value(&schema).unwrap_or(serde_json::json!({}));
        match value {
            serde_json::Value::Object(map) => Arc::new(map),
            _ => Arc::new(serde_json::Map::new()),
        }
    }

    /// Convert FossaError to McpError.
    fn to_mcp_error(err: FossaError) -> McpError {
        match &err {
            FossaError::NotFound { entity_type, id } => {
                McpError::resource_not_found(format!("{entity_type} '{id}' not found"), None)
            }
            FossaError::ConfigMissing(msg) => McpError::invalid_params(msg.clone(), None),
            FossaError::InvalidParams(msg) => McpError::invalid_params(msg.clone(), None),
            FossaError::InvalidLocator(loc) => {
                McpError::invalid_params(format!("Invalid locator: {loc}"), None)
            }
            _ => McpError::internal_error(err.to_string(), None),
        }
    }

    /// Append a migration hint when failed args look like the legacy (pre-0.3
    /// unification) shape: generic `parent` field / string `id`. Remove at 1.0.
    fn describe_args_error(err: serde_json::Error, args: &serde_json::Value) -> McpError {
        let legacy = args.get("parent").is_some()
            || matches!(args.get("id"), Some(serde_json::Value::String(_)));
        let msg = if legacy {
            format!(
                "{err} — note: the MCP arg shapes changed: per-entity fields \
                 replace the generic `parent`/string `id` (see the README's \
                 migration notes, or re-read this tool's input schema)"
            )
        } else {
            err.to_string()
        };
        McpError::invalid_params(msg, None)
    }

    /// Serialize an operation result into a tool response.
    fn to_result<T: Serialize>(value: &T) -> Result<CallToolResult, McpError> {
        let text = serde_json::to_string_pretty(value)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    /// Handle the `get` tool.
    pub async fn handle_get(&self, command: GetCommand) -> Result<CallToolResult, McpError> {
        let output = run_get(&self.client, command)
            .await
            .map_err(Self::to_mcp_error)?;
        Self::to_result(&output)
    }

    /// Handle the `list` tool.
    pub async fn handle_list(&self, command: ListCommand) -> Result<CallToolResult, McpError> {
        let output = run_list(&self.client, command)
            .await
            .map_err(Self::to_mcp_error)?;
        Self::to_result(&output)
    }

    /// Handle the `update` tool.
    pub async fn handle_update(&self, command: UpdateCommand) -> Result<CallToolResult, McpError> {
        let output = run_update(&self.client, command)
            .await
            .map_err(Self::to_mcp_error)?;
        Self::to_result(&output)
    }

    /// Handle the `ignore` tool.
    pub async fn handle_ignore(&self, command: IgnoreCommand) -> Result<CallToolResult, McpError> {
        let output = run_ignore(&self.client, command)
            .await
            .map_err(Self::to_mcp_error)?;
        Self::to_result(&output)
    }

    /// Handle the `unignore` tool.
    pub async fn handle_unignore(
        &self,
        command: UnignoreCommand,
    ) -> Result<CallToolResult, McpError> {
        let output = run_unignore(&self.client, command)
            .await
            .map_err(Self::to_mcp_error)?;
        Self::to_result(&output)
    }
}

impl ServerHandler for FossaServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "fossapi".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some(
                "FOSSA API MCP Server - Query projects, revisions, issues, dependencies, and \
                 snippet matches. Tools mirror the fossapi CLI verbs: each takes an `entity` \
                 discriminator plus that entity's parameters. Use \
                 list(entity: snippet_locations, revision: <revision locator>) to map \
                 third-party code matches to first-party files, then \
                 get(entity: snippet_match) to drill into a single match's first-party line \
                 numbers and side-by-side code."
                    .to_string(),
            ),
        }
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult {
            tools: Self::tools(),
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let args = request
            .arguments
            .map(serde_json::Value::Object)
            .unwrap_or(serde_json::json!({}));

        match request.name.as_ref() {
            "get" => {
                let command: GetCommand = serde_json::from_value(args.clone())
                    .map_err(|e| Self::describe_args_error(e, &args))?;
                self.handle_get(command).await
            }
            "list" => {
                let command: ListCommand = serde_json::from_value(args.clone())
                    .map_err(|e| Self::describe_args_error(e, &args))?;
                self.handle_list(command).await
            }
            "update" => {
                let command: UpdateCommand = serde_json::from_value(args.clone())
                    .map_err(|e| Self::describe_args_error(e, &args))?;
                self.handle_update(command).await
            }
            "ignore" => {
                let command: IgnoreCommand = serde_json::from_value(args.clone())
                    .map_err(|e| Self::describe_args_error(e, &args))?;
                self.handle_ignore(command).await
            }
            "unignore" => {
                let command: UnignoreCommand = serde_json::from_value(args.clone())
                    .map_err(|e| Self::describe_args_error(e, &args))?;
                self.handle_unignore(command).await
            }
            other => Err(McpError::invalid_params(
                format!("Unknown tool: {other}"),
                None,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::{
        GetIssueParams, GetProjectParams, GetRevisionParams, GetSnippetMatchParams,
        GetSnippetParams, IgnoreIssueParams, ListDependenciesParams, ListProjectsParams,
        ListRevisionsParams, ListSnippetLocationsParams, ListSnippetsParams, PageArgs,
        UnignoreIssueParams, UpdateProjectParams,
    };
    use crate::IssueCategory;
    use wiremock::matchers::{method, path, path_regex, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn response_text(result: &CallToolResult) -> &str {
        match &result.content[0].raw {
            rmcp::model::RawContent::Text(t) => &t.text,
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn tools_are_the_cli_verbs() {
        let names: Vec<_> = FossaServer::tools()
            .iter()
            .map(|t| t.name.to_string())
            .collect();
        assert_eq!(names, ["get", "list", "update", "ignore", "unignore"]);
    }

    #[test]
    fn schemas_generate_for_all_verbs() {
        assert!(!FossaServer::schema::<GetCommand>().is_empty());
        assert!(!FossaServer::schema::<ListCommand>().is_empty());
        assert!(!FossaServer::schema::<UpdateCommand>().is_empty());
        assert!(!FossaServer::schema::<IgnoreCommand>().is_empty());
        assert!(!FossaServer::schema::<UnignoreCommand>().is_empty());
    }

    #[test]
    fn server_info_has_correct_name() {
        // This compiles only if FossaServer implements ServerHandler correctly.
        fn assert_server_handler<T: ServerHandler>() {}
        assert_server_handler::<FossaServer>();
    }

    // =========================================================================
    // list handler tests
    // =========================================================================

    /// Test: list(entity: projects) returns paginated list
    #[tokio::test]
    async fn handle_list_projects_returns_paginated_list() {
        let mock_server = MockServer::start().await;

        let response = serde_json::json!({
            "projects": [
                {
                    "id": "custom+1/proj1",
                    "title": "Project 1",
                    "public": false,
                    "labels": [],
                    "teams": []
                },
                {
                    "id": "custom+1/proj2",
                    "title": "Project 2",
                    "public": false,
                    "labels": [],
                    "teams": []
                }
            ],
            "total": 2
        });

        Mock::given(method("GET"))
            .and(path("/v2/projects"))
            .and(query_param("page", "1"))
            .and(query_param("count", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let result = server
            .handle_list(ListCommand::Projects(ListProjectsParams {
                pagination: PageArgs::default(),
            }))
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
        let page: serde_json::Value = serde_json::from_str(response_text(&result)).unwrap();
        assert_eq!(page["items"].as_array().unwrap().len(), 2);
        assert_eq!(page["page"], 1);
        assert_eq!(page["count"], 20);
    }

    /// Test: list(entity: revisions, project: locator) lists revisions
    #[tokio::test]
    async fn handle_list_revisions_with_project() {
        let mock_server = MockServer::start().await;

        let response = serde_json::json!({
            "default_branch": {
                "revisions": [
                    {
                        "locator": "custom+org/repo$abc123",
                        "resolved": true,
                        "source": "cli",
                        "unresolved_issue_count": 0,
                        "unresolved_licensing_issue_count": 0,
                        "created_at": "2024-01-01T00:00:00Z"
                    }
                ]
            }
        });

        Mock::given(method("GET"))
            .and(path("/projects/custom%2Borg%2Frepo/revisions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let result = server
            .handle_list(ListCommand::Revisions(ListRevisionsParams {
                project: "custom+org/repo".to_string(),
                pagination: PageArgs::default(),
            }))
            .await
            .unwrap();
        assert!(!result.is_error.unwrap_or(false));
    }

    /// Test: list(entity: dependencies, revision: locator) lists deps
    #[tokio::test]
    async fn handle_list_dependencies_with_revision() {
        let mock_server = MockServer::start().await;

        let response = serde_json::json!({
            "dependencies": [
                {
                    "locator": "npm+lodash$4.17.21",
                    "depth": 1,
                    "licenses": ["MIT"]
                }
            ],
            "count": 1
        });

        Mock::given(method("GET"))
            .and(path(
                "/v2/revisions/custom%2Borg%2Frepo%24abc123/dependencies",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let result = server
            .handle_list(ListCommand::Dependencies(ListDependenciesParams {
                revision: "custom+org/repo$abc123".to_string(),
                pagination: PageArgs::default(),
            }))
            .await
            .unwrap();
        assert!(!result.is_error.unwrap_or(false));
    }

    /// Test: list JSON missing a required field fails to deserialize.
    #[test]
    fn list_revisions_json_without_project_is_rejected() {
        let err = serde_json::from_value::<ListCommand>(serde_json::json!({"entity": "revisions"}))
            .unwrap_err();
        assert!(err.to_string().contains("project"), "{err}");
    }

    /// Test: Count is capped at 100
    #[tokio::test]
    async fn handle_list_caps_count_at_100() {
        let mock_server = MockServer::start().await;

        // Request count=200, should be capped to 100
        Mock::given(method("GET"))
            .and(path("/v2/projects"))
            .and(query_param("page", "1"))
            .and(query_param("count", "100")) // Capped from 200
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "projects": [],
                "total": 0
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let _ = server
            .handle_list(ListCommand::Projects(ListProjectsParams {
                pagination: PageArgs {
                    page: Some(1),
                    count: Some(200),
                },
            }))
            .await;
        // Mock expectations verify count was capped
    }

    // =========================================================================
    // get handler tests
    // =========================================================================

    #[tokio::test]
    async fn handle_get_project_returns_json() {
        let mock_server = MockServer::start().await;

        let project_json = serde_json::json!({
            "id": "custom+123/test-project",
            "title": "Test Project",
            "public": false,
            "labels": [],
            "teams": []
        });

        Mock::given(method("GET"))
            .and(path("/projects/custom%2B123%2Ftest-project"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&project_json))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let result = server
            .handle_get(GetCommand::Project(GetProjectParams {
                locator: "custom+123/test-project".to_string(),
            }))
            .await
            .expect("handle_get should succeed");

        assert!(!result.is_error.unwrap_or(false));
        let text = response_text(&result);
        assert!(text.contains("Test Project"));
        assert!(text.contains("custom+123/test-project"));
    }

    #[tokio::test]
    async fn handle_get_revision_returns_json() {
        let mock_server = MockServer::start().await;

        let revision_json = serde_json::json!({
            "locator": "custom+123/test$main",
            "resolved": true,
            "sourceType": "cargo"
        });

        Mock::given(method("GET"))
            .and(path("/revisions/custom%2B123%2Ftest%24main"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&revision_json))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let result = server
            .handle_get(GetCommand::Revision(GetRevisionParams {
                locator: "custom+123/test$main".to_string(),
            }))
            .await
            .expect("handle_get should succeed");

        assert!(!result.is_error.unwrap_or(false));
        let text = response_text(&result);
        assert!(text.contains("custom+123/test$main"));
        assert!(text.contains("resolved"));
    }

    #[tokio::test]
    async fn handle_get_issue_returns_json() {
        let mock_server = MockServer::start().await;

        let issue_json = serde_json::json!({
            "id": 12345,
            "type": "vulnerability",
            "source": {"id": "npm+lodash$4.17.0"},
            "depths": {"direct": 1, "deep": 0},
            "statuses": {"active": 1, "ignored": 0},
            "projects": [],
            "cve": "CVE-2024-0001",
            "severity": "high"
        });

        Mock::given(method("GET"))
            .and(path("/v2/issues/12345"))
            .and(query_param("category", "vulnerability"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&issue_json))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let result = server
            .handle_get(GetCommand::Issue(GetIssueParams {
                id: 12345,
                category: Some(IssueCategory::Vulnerability),
            }))
            .await
            .expect("handle_get should succeed");

        assert!(!result.is_error.unwrap_or(false));
        let text = response_text(&result);
        assert!(text.contains("12345"));
        assert!(text.contains("vulnerability"));
        assert!(text.contains("CVE-2024-0001"));
    }

    /// Without a category, the handler probes categories instead of erroring.
    #[tokio::test]
    async fn handle_get_issue_without_category_probes() {
        let mock_server = MockServer::start().await;

        let issue_json = serde_json::json!({
            "id": 12345,
            "type": "licensing",
            "source": {"id": "npm+leftpad$1.0.0"},
            "depths": {"direct": 1, "deep": 0},
            "statuses": {"active": 1, "ignored": 0},
            "projects": [],
            "license": "GPL-3.0"
        });

        // First probed category answers 404; the next answers 200.
        Mock::given(method("GET"))
            .and(path("/v2/issues/12345"))
            .and(query_param("category", "vulnerability"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v2/issues/12345"))
            .and(query_param("category", "licensing"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&issue_json))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let result = server
            .handle_get(GetCommand::Issue(GetIssueParams {
                id: 12345,
                category: None,
            }))
            .await
            .expect("handle_get should succeed via category probe");

        assert!(!result.is_error.unwrap_or(false));
        assert!(response_text(&result).contains("GPL-3.0"));
    }

    // =========================================================================
    // update handler tests
    // =========================================================================

    #[tokio::test]
    async fn handle_update_project_title_succeeds() {
        use wiremock::matchers::body_json;

        let mock_server = MockServer::start().await;

        let expected_body = serde_json::json!({
            "title": "Updated Title"
        });

        let response_project = serde_json::json!({
            "id": "custom+acme/myapp",
            "title": "Updated Title",
            "public": false,
            "labels": [],
            "teams": []
        });

        Mock::given(method("PUT"))
            .and(path("/projects/custom%2Bacme%2Fmyapp"))
            .and(body_json(&expected_body))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_project))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let result = server
            .handle_update(UpdateCommand::Project(UpdateProjectParams {
                locator: "custom+acme/myapp".to_string(),
                title: Some("Updated Title".to_string()),
                description: None,
                url: None,
                public: None,
                policy_id: None,
                default_branch: None,
            }))
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
        assert!(response_text(&result).contains("Updated Title"));
    }

    #[tokio::test]
    async fn handle_update_project_url_succeeds() {
        use wiremock::matchers::body_json;

        let mock_server = MockServer::start().await;

        let expected_body = serde_json::json!({
            "url": "https://example.com/repo"
        });

        let response_project = serde_json::json!({
            "id": "custom+acme/myapp",
            "title": "My App",
            "public": false,
            "labels": [],
            "teams": []
        });

        Mock::given(method("PUT"))
            .and(path("/projects/custom%2Bacme%2Fmyapp"))
            .and(body_json(&expected_body))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_project))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let result = server
            .handle_update(UpdateCommand::Project(UpdateProjectParams {
                locator: "custom+acme/myapp".to_string(),
                title: None,
                description: None,
                url: Some("https://example.com/repo".to_string()),
                public: None,
                policy_id: None,
                default_branch: None,
            }))
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
        assert!(response_text(&result).contains("custom+acme/myapp"));
    }

    // =========================================================================
    // snippet handler tests
    // =========================================================================

    /// Test: list(entity: snippets) returns the paged snippet summaries.
    #[tokio::test]
    async fn handle_list_snippets_returns_page() {
        let mock_server = MockServer::start().await;

        let list_body = serde_json::json!({
            "results": [{
                "id": "55", "packageId": "7", "purl": "pkg:x",
                "locator": "pod+x$1", "package": "X", "version": "1.0",
                "kind": "file", "matchCount": 1
            }],
            "totalCount": 1, "page": 1, "pageSize": 50
        });

        Mock::given(method("GET"))
            .and(path("/revisions/custom%2Borg%2Frepo%24main/snippets"))
            .and(query_param("page", "1"))
            .and(query_param("pageSize", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&list_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let result = server
            .handle_list(ListCommand::Snippets(ListSnippetsParams {
                revision: "custom+org/repo$main".to_string(),
                path: None,
                pagination: PageArgs::default(),
            }))
            .await
            .unwrap();
        assert!(!result.is_error.unwrap_or(false));
        let page: serde_json::Value = serde_json::from_str(response_text(&result)).unwrap();
        assert_eq!(page["items"][0]["id"], "55");
    }

    /// Test: list(entity: snippet_locations) returns flattened locations.
    #[tokio::test]
    async fn handle_list_snippet_locations_returns_locations() {
        let mock_server = MockServer::start().await;

        let list_body = serde_json::json!({
            "results": [{
                "id": "55", "packageId": "7", "purl": "pkg:x",
                "locator": "pod+x$1", "package": "X", "version": "1.0",
                "kind": "file", "matchCount": 1
            }],
            "totalCount": 1, "page": 1, "pageSize": 50
        });
        let details_body = serde_json::json!({
            "snippet": {
                "id": "55", "packageId": "7", "purl": "pkg:x",
                "locator": "pod+x$1", "package": "X", "version": "1.0",
                "kind": "file",
                "matches": [{"path": "/src/a.rs", "matchPercentage": 1}]
            }
        });

        Mock::given(method("GET"))
            .and(path("/revisions/custom%2Borg%2Frepo%24main/snippets"))
            .and(query_param("page", "1"))
            .and(query_param("pageSize", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&list_body))
            .expect(1)
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/revisions/custom%2Borg%2Frepo%24main/snippets/55"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&details_body))
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let result = server
            .handle_list(ListCommand::SnippetLocations(ListSnippetLocationsParams {
                revision: "custom+org/repo$main".to_string(),
                path: None,
                with_lines: false,
                pagination: PageArgs::default(),
            }))
            .await
            .unwrap();
        assert!(!result.is_error.unwrap_or(false));
        let page: serde_json::Value = serde_json::from_str(response_text(&result)).unwrap();
        assert_eq!(page["items"][0]["path"], "/src/a.rs");
        assert_eq!(page["items"][0]["snippet_id"], "55");
        assert_eq!(page["page"], 1);
    }

    /// Test: update with no fields is rejected before any HTTP call.
    #[tokio::test]
    async fn handle_update_project_without_fields_is_rejected() {
        let mock_server = MockServer::start().await;
        // No PUT mock mounted: a request would fail the test via connection to
        // an unmatched route below anyway, but the point is we never get there.

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let err = server
            .handle_update(UpdateCommand::Project(UpdateProjectParams {
                locator: "custom+acme/myapp".to_string(),
                title: None,
                description: None,
                url: None,
                public: None,
                policy_id: None,
                default_branch: None,
            }))
            .await
            .unwrap_err();
        assert!(err.message.contains("at least one field"), "{err:?}");
    }

    /// Test: legacy arg shapes get a migration hint appended to the error.
    #[tokio::test]
    async fn call_tool_legacy_args_get_migration_hint() {
        let err = FossaServer::describe_args_error(
            serde_json::from_value::<GetCommand>(
                serde_json::json!({"entity": "issue", "id": "123", "parent": "x"}),
            )
            .unwrap_err(),
            &serde_json::json!({"entity": "issue", "id": "123", "parent": "x"}),
        );
        assert!(err.message.contains("arg shapes changed"), "{err:?}");
    }

    /// Test: get(entity: snippet) returns snippet details with matches.
    #[tokio::test]
    async fn handle_get_snippet_returns_details() {
        let mock_server = MockServer::start().await;

        let details_body = serde_json::json!({
            "snippet": {
                "id": "55", "packageId": "7", "purl": "pkg:x",
                "locator": "pod+x$1", "package": "X", "version": "1.0",
                "kind": "file",
                "matches": [{"path": "/src/a.rs", "matchPercentage": 1}]
            }
        });

        Mock::given(method("GET"))
            .and(path("/revisions/custom%2Borg%2Frepo%24main/snippets/55"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&details_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let result = server
            .handle_get(GetCommand::Snippet(GetSnippetParams {
                revision: "custom+org/repo$main".to_string(),
                snippet: "55".to_string(),
            }))
            .await
            .unwrap();
        assert!(!result.is_error.unwrap_or(false));
        assert!(response_text(&result).contains("/src/a.rs"));
    }

    /// Test: get(entity: snippet_match) drills into a single match.
    #[tokio::test]
    async fn handle_get_snippet_match_returns_code() {
        let mock_server = MockServer::start().await;

        let match_body = serde_json::json!({
            "matchDetails": {
                "path": "/src/a.rs",
                "matchPercentage": 100,
                "referenceCode": [{"line": "x", "lineNumber": 1, "isHighlighted": true}],
                "detectedCode": [{"line": "x", "lineNumber": 42, "isHighlighted": true}]
            }
        });

        Mock::given(method("GET"))
            .and(path_regex(r"/snippets/55/matches/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&match_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let result = server
            .handle_get(GetCommand::SnippetMatch(GetSnippetMatchParams {
                revision: "custom+org/repo$main".to_string(),
                snippet: "55".to_string(),
                path: "/src/a.rs".to_string(),
            }))
            .await
            .unwrap();
        assert!(!result.is_error.unwrap_or(false));
        let text = response_text(&result);
        assert!(text.contains("detectedCode") || text.contains("detected_code"));
        assert!(text.contains("42"));
    }

    // =========================================================================
    // ignore / unignore handler tests
    // =========================================================================

    fn issue_body(id: u64, active: u32, ignored: u32) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "type": "licensing",
            "source": {"id": "npm+leftpad$1.0.0"},
            "statuses": {"active": active, "ignored": ignored},
            "license": "GPL-3.0"
        })
    }

    fn ignore_licensing_issue() -> IgnoreCommand {
        IgnoreCommand::Issue(IgnoreIssueParams {
            id: 987654,
            category: IssueCategory::Licensing,
            notes: Some("false positive patch".to_string()),
            reason: None,
        })
    }

    /// Test: ignore fetches the issue, sends the action, and returns the
    /// refreshed issue.
    #[tokio::test]
    async fn handle_ignore_issue_succeeds() {
        use wiremock::matchers::body_json;

        let mock_server = MockServer::start().await;

        // Pre-flight guard fetch (active) and post-write refresh (ignored).
        Mock::given(method("GET"))
            .and(path("/v2/issues/987654"))
            .and(query_param("category", "licensing"))
            .respond_with(ResponseTemplate::new(200).set_body_json(issue_body(987654, 1, 0)))
            .up_to_n_times(1)
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("PUT"))
            .and(path("/v2/issues/"))
            .and(query_param("category", "licensing"))
            .and(query_param("ids[]", "987654"))
            .and(body_json(serde_json::json!({
                "type": "ignore",
                "notes": "false positive patch"
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"count": 1, "issueId": 987654})),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v2/issues/987654"))
            .and(query_param("category", "licensing"))
            .respond_with(ResponseTemplate::new(200).set_body_json(issue_body(987654, 0, 1)))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let result = server
            .handle_ignore(ignore_licensing_issue())
            .await
            .unwrap();
        assert!(!result.is_error.unwrap_or(false));
        let text = response_text(&result);
        assert!(text.contains("987654"));
        assert!(text.contains("\"ignored\": 1"));
    }

    /// Test: ignoring an already-ignored issue is refused before any PUT.
    #[tokio::test]
    async fn handle_ignore_already_ignored_is_rejected_without_put() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/issues/987654"))
            .respond_with(ResponseTemplate::new(200).set_body_json(issue_body(987654, 0, 1)))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("PUT"))
            .and(path("/v2/issues/"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let err = server
            .handle_ignore(ignore_licensing_issue())
            .await
            .unwrap_err();
        assert!(err.message.contains("unignore it first"), "{err:?}");
    }

    /// Test: unignoring an issue with nothing ignored is refused before any PUT.
    #[tokio::test]
    async fn handle_unignore_active_is_rejected_without_put() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/issues/987654"))
            .respond_with(ResponseTemplate::new(200).set_body_json(issue_body(987654, 1, 0)))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("PUT"))
            .and(path("/v2/issues/"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let err = server
            .handle_unignore(UnignoreCommand::Issue(UnignoreIssueParams {
                id: 987654,
                category: IssueCategory::Licensing,
            }))
            .await
            .unwrap_err();
        assert!(err.message.contains("nothing to unignore"), "{err:?}");
    }

    /// Test: a partially ignored issue (org-wide rollup) accepts both actions,
    /// like the UI's global issue view.
    #[tokio::test]
    async fn handle_ignore_partial_state_is_allowed() {
        use wiremock::matchers::body_json;

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/issues/987654"))
            .respond_with(ResponseTemplate::new(200).set_body_json(issue_body(987654, 2, 1)))
            .up_to_n_times(1)
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("PUT"))
            .and(path("/v2/issues/"))
            .and(body_json(serde_json::json!({
                "type": "ignore",
                "notes": "false positive patch"
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"count": 2, "issueId": null})),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v2/issues/987654"))
            .respond_with(ResponseTemplate::new(200).set_body_json(issue_body(987654, 0, 3)))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let result = server
            .handle_ignore(ignore_licensing_issue())
            .await
            .unwrap();
        assert!(!result.is_error.unwrap_or(false));
    }

    /// Test: a reason on a non-vulnerability ignore is refused before any
    /// HTTP call — the server would store it, but nothing ever displays it.
    #[tokio::test]
    async fn handle_ignore_reason_on_licensing_is_rejected() {
        let client = FossaClient::new("test-token", "http://localhost:9999").unwrap();
        let server = FossaServer::new(client);

        let err = server
            .handle_ignore(IgnoreCommand::Issue(IgnoreIssueParams {
                id: 987654,
                category: IssueCategory::Licensing,
                notes: None,
                reason: Some(crate::IssueIgnoreReason::Other),
            }))
            .await
            .unwrap_err();
        assert!(
            err.message.contains("only apply to vulnerability ignores"),
            "{err:?}"
        );
    }
}
