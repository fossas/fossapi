//! MCP tool parameter types with JSON Schema support.

use schemars::JsonSchema;
use serde::Deserialize;

use crate::IssueCategory;

/// Entity types supported by MCP tools.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    /// FOSSA project.
    Project,
    /// Project revision.
    Revision,
    /// Security, licensing, or quality issue.
    Issue,
    /// Package dependency.
    Dependency,
}

/// Parameters for the `get` MCP tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetParams {
    /// The type of entity to fetch.
    pub entity: EntityType,
    /// The entity identifier (locator or ID).
    pub id: String,
}

/// Parameters for the `list` MCP tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ListParams {
    /// The type of entity to list.
    pub entity: EntityType,
    /// Parent entity locator (required for Revision and Dependency).
    #[serde(default)]
    pub parent: Option<String>,
    /// Page number (1-indexed).
    #[serde(default)]
    pub page: Option<u32>,
    /// Number of items per page (max 100).
    #[serde(default)]
    pub count: Option<u32>,
    /// Issue category filter (required for Issue entity: vulnerability, licensing, quality).
    #[serde(default)]
    pub category: Option<IssueCategory>,
}

/// Parameters for the `update` MCP tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct UpdateParams {
    /// The type of entity to update.
    pub entity: EntityType,
    /// The entity locator.
    pub locator: String,
    /// New title (Project only).
    #[serde(default)]
    pub title: Option<String>,
    /// New description (Project only).
    #[serde(default)]
    pub description: Option<String>,
    /// New URL (Project only).
    #[serde(default)]
    pub url: Option<String>,
    /// Whether the project is public (Project only).
    #[serde(default)]
    pub public: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_params_schema_generates() {
        let schema = schemars::schema_for!(GetParams);
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("entity"));
        assert!(json.contains("id"));
    }

    #[test]
    fn list_params_schema_generates() {
        let schema = schemars::schema_for!(ListParams);
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("entity"));
        assert!(json.contains("parent"));
        assert!(json.contains("page"));
        assert!(json.contains("count"));
        assert!(json.contains("category"));
    }

    #[test]
    fn update_params_schema_generates() {
        let schema = schemars::schema_for!(UpdateParams);
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("entity"));
        assert!(json.contains("locator"));
        assert!(json.contains("title"));
    }

    #[test]
    fn entity_type_schema_has_variants() {
        let schema = schemars::schema_for!(EntityType);
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("project"));
        assert!(json.contains("revision"));
        assert!(json.contains("issue"));
        assert!(json.contains("dependency"));
    }

    #[test]
    fn get_params_deserializes() {
        let json = r#"{"entity": "project", "id": "custom+org/repo"}"#;
        let params: GetParams = serde_json::from_str(json).unwrap();
        assert!(matches!(params.entity, EntityType::Project));
        assert_eq!(params.id, "custom+org/repo");
    }

    #[test]
    fn list_params_deserializes_with_defaults() {
        let json = r#"{"entity": "revision"}"#;
        let params: ListParams = serde_json::from_str(json).unwrap();
        assert!(matches!(params.entity, EntityType::Revision));
        assert!(params.parent.is_none());
        assert!(params.page.is_none());
        assert!(params.count.is_none());
        assert!(params.category.is_none());
    }

    #[test]
    fn list_params_deserializes_with_all_fields() {
        let json = r#"{"entity": "dependency", "parent": "custom+org/repo$main", "page": 2, "count": 50}"#;
        let params: ListParams = serde_json::from_str(json).unwrap();
        assert!(matches!(params.entity, EntityType::Dependency));
        assert_eq!(params.parent, Some("custom+org/repo$main".to_string()));
        assert_eq!(params.page, Some(2));
        assert_eq!(params.count, Some(50));
    }

    #[test]
    fn update_params_deserializes() {
        let json = r#"{"entity": "project", "locator": "custom+org/repo", "title": "New Title"}"#;
        let params: UpdateParams = serde_json::from_str(json).unwrap();
        assert!(matches!(params.entity, EntityType::Project));
        assert_eq!(params.locator, "custom+org/repo");
        assert_eq!(params.title, Some("New Title".to_string()));
        assert!(params.description.is_none());
    }

    #[test]
    fn list_params_deserializes_with_category() {
        let json = r#"{"entity": "issue", "category": "vulnerability"}"#;
        let params: ListParams = serde_json::from_str(json).unwrap();
        assert!(matches!(params.entity, EntityType::Issue));
        assert!(matches!(params.category, Some(IssueCategory::Vulnerability)));
    }

    #[test]
    fn list_params_schema_includes_category() {
        let schema = schemars::schema_for!(ListParams);
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("category"));
    }
}
