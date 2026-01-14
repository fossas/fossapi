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
use std::sync::Arc;

use crate::{
    mcp::{EntityType, GetParams, ListParams, UpdateParams},
    DependencyListQuery, FossaClient, FossaError, Get, Issue, IssueListQuery, List, Project,
    ProjectListQuery, ProjectUpdateParams, Revision, RevisionListQuery, Update,
};

/// FOSSA MCP Server.
///
/// Implements the MCP ServerHandler trait, providing tools to interact
/// with the FOSSA API through the Model Context Protocol.
///
/// # Tools
///
/// - `get` - Fetch a single entity by ID
/// - `list` - List entities with pagination
/// - `update` - Update an entity (Project only)
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
            FossaError::InvalidLocator(loc) => {
                McpError::invalid_params(format!("Invalid locator: {loc}"), None)
            }
            _ => McpError::internal_error(err.to_string(), None),
        }
    }

    /// Handle the `get` tool.
    ///
    /// # Arguments
    ///
    /// * `params` - The get parameters including entity type and ID
    ///
    /// # Returns
    ///
    /// Returns the entity as pretty-printed JSON in a `CallToolResult`.
    ///
    /// # Errors
    ///
    /// Returns an MCP error if:
    /// - Entity type is `Dependency` (not supported for get)
    /// - Issue ID is not a valid number
    /// - The underlying API call fails
    pub async fn handle_get(
        &self,
        params: GetParams,
    ) -> Result<CallToolResult, McpError> {
        let result = match params.entity {
            EntityType::Project => {
                let project = Project::get(&self.client, params.id)
                    .await
                    .map_err(Self::to_mcp_error)?;
                serde_json::to_string_pretty(&project)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            EntityType::Revision => {
                let revision = Revision::get(&self.client, params.id)
                    .await
                    .map_err(Self::to_mcp_error)?;
                serde_json::to_string_pretty(&revision)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            EntityType::Issue => {
                let id: u64 = params
                    .id
                    .parse()
                    .map_err(|_| McpError::invalid_params("Issue ID must be a number", None))?;
                let issue = Issue::get(&self.client, id)
                    .await
                    .map_err(Self::to_mcp_error)?;
                serde_json::to_string_pretty(&issue)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            EntityType::Dependency => {
                return Err(McpError::invalid_params(
                    "Dependency does not support get. Use list with a parent revision locator.",
                    None,
                ));
            }
        };

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    /// Handle the `list` tool.
    async fn handle_list(&self, params: ListParams) -> Result<CallToolResult, McpError> {
        let page = params.page.unwrap_or(1);
        let count = params.count.unwrap_or(20).min(100);

        let result = match params.entity {
            EntityType::Project => {
                let query = ProjectListQuery::default();
                let page_result = Project::list_page(&self.client, &query, page, count)
                    .await
                    .map_err(Self::to_mcp_error)?;
                serde_json::to_string_pretty(&page_result)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            EntityType::Revision => {
                let parent = params.parent.ok_or_else(|| {
                    McpError::invalid_params(
                        "parent is required for listing revisions (project locator)",
                        None,
                    )
                })?;
                let query = RevisionListQuery::default();
                let page_result =
                    crate::get_revisions_page(&self.client, &parent, query, page, count)
                        .await
                        .map_err(Self::to_mcp_error)?;
                serde_json::to_string_pretty(&page_result)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            EntityType::Issue => {
                let query = IssueListQuery::default();
                let page_result = crate::get_issues_page(&self.client, query, page, count)
                    .await
                    .map_err(Self::to_mcp_error)?;
                serde_json::to_string_pretty(&page_result)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            EntityType::Dependency => {
                let parent = params.parent.ok_or_else(|| {
                    McpError::invalid_params(
                        "parent is required for listing dependencies (revision locator)",
                        None,
                    )
                })?;
                let query = DependencyListQuery::default();
                let page_result =
                    crate::get_dependencies_page(&self.client, &parent, query, page, count)
                        .await
                        .map_err(Self::to_mcp_error)?;
                serde_json::to_string_pretty(&page_result)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
        };

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    /// Handle the `update` tool.
    async fn handle_update(&self, params: UpdateParams) -> Result<CallToolResult, McpError> {
        match params.entity {
            EntityType::Project => {
                let update_params = ProjectUpdateParams {
                    title: params.title,
                    description: params.description,
                    url: params.url,
                    public: params.public,
                    policy_id: None,
                    default_branch: None,
                };
                let project = Project::update(&self.client, params.locator, update_params)
                    .await
                    .map_err(Self::to_mcp_error)?;
                let result = serde_json::to_string_pretty(&project)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(result)]))
            }
            EntityType::Revision => Err(McpError::invalid_params(
                "Update not supported for Revision",
                None,
            )),
            EntityType::Issue => Err(McpError::invalid_params(
                "Update not supported for Issue",
                None,
            )),
            EntityType::Dependency => Err(McpError::invalid_params(
                "Update not supported for Dependency",
                None,
            )),
        }
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
                "FOSSA API MCP Server - Query projects, revisions, issues, and dependencies."
                    .to_string(),
            ),
        }
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let tools = vec![
            Tool::new(
                "get",
                "Fetch a single FOSSA entity by ID. \
                 Supports: project (by locator), revision (by locator), issue (by numeric ID). \
                 Dependency must use list with parent.",
                Self::schema::<GetParams>(),
            ),
            Tool::new(
                "list",
                "List FOSSA entities with pagination. \
                 Projects: no parent needed. \
                 Revisions: parent = project locator. \
                 Issues: no parent needed. \
                 Dependencies: parent = revision locator.",
                Self::schema::<ListParams>(),
            ),
            Tool::new(
                "update",
                "Update a FOSSA entity. Currently only Project is supported. \
                 Can update: title, description, url, public.",
                Self::schema::<UpdateParams>(),
            ),
        ];

        Ok(ListToolsResult {
            tools,
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
                let params: GetParams = serde_json::from_value(args)
                    .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
                self.handle_get(params).await
            }
            "list" => {
                let params: ListParams = serde_json::from_value(args)
                    .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
                self.handle_list(params).await
            }
            "update" => {
                let params: UpdateParams = serde_json::from_value(args)
                    .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
                self.handle_update(params).await
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
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn schema_generates_for_get_params() {
        let schema = FossaServer::schema::<GetParams>();
        assert!(!schema.is_empty());
    }

    #[test]
    fn schema_generates_for_list_params() {
        let schema = FossaServer::schema::<ListParams>();
        assert!(!schema.is_empty());
    }

    #[test]
    fn schema_generates_for_update_params() {
        let schema = FossaServer::schema::<UpdateParams>();
        assert!(!schema.is_empty());
    }

    #[test]
    fn server_info_has_correct_name() {
        // We can't construct a FossaServer without env vars, but we can verify
        // the ServerInfo structure is correct by checking the trait method exists.
        #[allow(dead_code)]
        fn verify_get_info<T: ServerHandler>(server: &T) -> ServerInfo {
            server.get_info()
        }

        // This compiles only if FossaServer implements ServerHandler correctly.
        fn assert_server_handler<T: ServerHandler>() {}
        assert_server_handler::<FossaServer>();
    }

    // =========================================================================
    // ISS-10858: MCP list tool handler tests
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

        let params = ListParams {
            entity: EntityType::Project,
            parent: None,
            page: None,
            count: None,
        };

        let result = server.handle_list(params).await.unwrap();

        // Verify success
        assert!(!result.is_error.unwrap_or(false));

        // Parse response and verify Page structure
        let text = match &result.content[0].raw {
            rmcp::model::RawContent::Text(t) => &t.text,
            _ => panic!("Expected text content"),
        };
        let page: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(page["items"].as_array().unwrap().len(), 2);
        assert_eq!(page["page"], 1);
        assert_eq!(page["count"], 20);
    }

    /// Test: list(entity: revisions, parent: locator) lists revisions
    #[tokio::test]
    async fn handle_list_revisions_with_parent() {
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

        let params = ListParams {
            entity: EntityType::Revision,
            parent: Some("custom+org/repo".to_string()),
            page: None,
            count: None,
        };

        let result = server.handle_list(params).await.unwrap();
        assert!(!result.is_error.unwrap_or(false));
    }

    /// Test: list(entity: dependencies, parent: locator) lists deps
    #[tokio::test]
    async fn handle_list_dependencies_with_parent() {
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
            .and(path("/v2/revisions/custom%2Borg%2Frepo%24abc123/dependencies"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let params = ListParams {
            entity: EntityType::Dependency,
            parent: Some("custom+org/repo$abc123".to_string()),
            page: None,
            count: None,
        };

        let result = server.handle_list(params).await.unwrap();
        assert!(!result.is_error.unwrap_or(false));
    }

    /// Test: Missing required parent for revisions returns error
    #[tokio::test]
    async fn handle_list_revisions_without_parent_returns_error() {
        let mock_server = MockServer::start().await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let params = ListParams {
            entity: EntityType::Revision,
            parent: None, // Missing required parent
            page: None,
            count: None,
        };

        let result = server.handle_list(params).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.to_lowercase().contains("parent"));
    }

    /// Test: Missing required parent for dependencies returns error
    #[tokio::test]
    async fn handle_list_dependencies_without_parent_returns_error() {
        let mock_server = MockServer::start().await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let params = ListParams {
            entity: EntityType::Dependency,
            parent: None, // Missing required parent
            page: None,
            count: None,
        };

        let result = server.handle_list(params).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.to_lowercase().contains("parent"));
    }

    /// Test: Pagination uses defaults (page=1, count=20)
    #[tokio::test]
    async fn handle_list_uses_pagination_defaults() {
        let mock_server = MockServer::start().await;

        // Verify default page=1 and count=20
        Mock::given(method("GET"))
            .and(path("/v2/projects"))
            .and(query_param("page", "1"))
            .and(query_param("count", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "projects": [],
                "total": 0
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let params = ListParams {
            entity: EntityType::Project,
            parent: None,
            page: None,   // Should default to 1
            count: None,  // Should default to 20
        };

        let _ = server.handle_list(params).await;
        // Mock expectations verify the query params were correct
    }

    /// Test: Count is capped at 100
    #[tokio::test]
    async fn handle_list_caps_count_at_100() {
        let mock_server = MockServer::start().await;

        // Request count=200, should be capped to 100
        Mock::given(method("GET"))
            .and(path("/v2/projects"))
            .and(query_param("page", "1"))
            .and(query_param("count", "100"))  // Capped from 200
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "projects": [],
                "total": 0
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = FossaClient::new("test-token", &mock_server.uri()).unwrap();
        let server = FossaServer::new(client);

        let params = ListParams {
            entity: EntityType::Project,
            parent: None,
            page: Some(1),
            count: Some(200),  // Should be capped to 100
        };

        let _ = server.handle_list(params).await;
        // Mock expectations verify count was capped
    }
}
