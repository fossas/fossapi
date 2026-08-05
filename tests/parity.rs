//! CLI ⇄ MCP parity tests.
//!
//! The CLI and the MCP server both consume the shared operation enums in
//! `fossapi::ops`, so they cannot express different operation sets. These
//! tests guard the edges that the type system does not: a `#[serde(skip)]`
//! or `#[clap(skip)]` sneaking onto one surface, a flattened struct not
//! inlining into the schema, or the MCP tool list drifting from the verb set.

use clap::CommandFactory;
use fossapi::cli::Cli;
use fossapi::mcp::FossaServer;
use fossapi::ops::{GetCommand, ListCommand, UpdateCommand};
use schemars::schema_for;
use std::collections::BTreeSet;

/// The verbs shared by both surfaces: (CLI subcommand, schema of the shared enum).
fn verbs() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        (
            "get",
            serde_json::to_value(schema_for!(GetCommand)).unwrap(),
        ),
        (
            "list",
            serde_json::to_value(schema_for!(ListCommand)).unwrap(),
        ),
        (
            "update",
            serde_json::to_value(schema_for!(UpdateCommand)).unwrap(),
        ),
    ]
}

/// Leaf subcommands of a CLI verb, as clap sees them (kebab-case).
fn cli_leaves(verb: &str) -> Vec<clap::Command> {
    let root = Cli::command();
    let verb_cmd = root
        .get_subcommands()
        .find(|c| c.get_name() == verb)
        .unwrap_or_else(|| panic!("CLI is missing the `{verb}` verb"))
        .clone();
    verb_cmd
        .get_subcommands()
        .filter(|c| c.get_name() != "help")
        .cloned()
        .collect()
}

/// The `entity` tag values of a shared verb enum, extracted from its schema.
fn schema_entities(schema: &serde_json::Value) -> BTreeSet<String> {
    variant_schemas(schema).iter().map(entity_tag).collect()
}

/// The per-variant subschemas of an internally-tagged enum schema.
fn variant_schemas(schema: &serde_json::Value) -> Vec<serde_json::Value> {
    schema["oneOf"]
        .as_array()
        .unwrap_or_else(|| panic!("expected oneOf in schema: {schema:#}"))
        .clone()
}

/// The `entity` tag value of one variant subschema.
fn entity_tag(variant: &serde_json::Value) -> String {
    variant["properties"]["entity"]["enum"][0]
        .as_str()
        .unwrap_or_else(|| panic!("variant schema has no entity tag: {variant:#}"))
        .to_string()
}

/// Property names of one variant subschema, excluding the `entity` tag.
fn schema_properties(variant: &serde_json::Value) -> BTreeSet<String> {
    variant["properties"]
        .as_object()
        .expect("variant schema has properties")
        .keys()
        .filter(|k| k.as_str() != "entity")
        .cloned()
        .collect()
}

/// Argument ids of a clap leaf, excluding globals and auto-generated args.
fn cli_args(leaf: &clap::Command) -> BTreeSet<String> {
    leaf.get_arguments()
        .filter(|a| !a.is_global_set())
        .map(|a| a.get_id().to_string())
        .filter(|id| id != "help" && id != "version")
        .collect()
}

/// The MCP tool list is exactly the CLI verb set (plus nothing).
#[test]
fn mcp_tools_match_cli_verbs() {
    let tool_names: BTreeSet<String> = FossaServer::tools()
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    let verb_names: BTreeSet<String> = verbs().iter().map(|(v, _)| v.to_string()).collect();
    assert_eq!(
        tool_names, verb_names,
        "MCP tools and CLI verbs have drifted"
    );
}

/// Every CLI leaf subcommand is an MCP entity of the same verb, and vice
/// versa. Fails naming the offending subcommand if someone adds an operation
/// to one surface only.
#[test]
fn cli_subcommands_match_schema_entities() {
    for (verb, schema) in verbs() {
        let cli_names: BTreeSet<String> = cli_leaves(verb)
            .iter()
            .map(|c| c.get_name().replace('-', "_"))
            .collect();
        let schema_names = schema_entities(&schema);
        assert!(
            !cli_names.is_empty(),
            "`{verb}` has no CLI subcommands — extraction is broken"
        );
        assert_eq!(
            cli_names, schema_names,
            "`{verb}` operations differ between CLI subcommands and MCP schema entities"
        );
    }
}

/// Every operation exposes the same parameters on both surfaces: the clap
/// argument ids equal the schema property names. Catches an op wired to
/// different param structs, and flattened structs failing to inline.
#[test]
fn cli_args_match_schema_properties() {
    for (verb, schema) in verbs() {
        let variants = variant_schemas(&schema);
        for leaf in cli_leaves(verb) {
            let entity = leaf.get_name().replace('-', "_");
            let variant = variants
                .iter()
                .find(|v| entity_tag(v) == entity)
                .unwrap_or_else(|| panic!("no schema variant for `{verb} {entity}`"));
            assert_eq!(
                cli_args(&leaf),
                schema_properties(variant),
                "`{verb} {entity}` parameters differ between CLI args and MCP schema properties"
            );
        }
    }
}

/// Pagination args flatten into the schema as inline properties (schemars
/// must inline `#[serde(flatten)]` for the MCP arg shape to stay flat).
#[test]
fn page_args_inline_into_list_schemas() {
    let schema = serde_json::to_value(schema_for!(ListCommand)).unwrap();
    let variants = variant_schemas(&schema);
    let projects = variants
        .iter()
        .find(|v| entity_tag(v) == "projects")
        .expect("projects variant");
    let props = schema_properties(projects);
    assert!(
        props.contains("page") && props.contains("count"),
        "PageArgs fields must inline into the variant schema, got: {props:?}"
    );
}

/// The MCP wire format round-trips: one tagged payload per verb deserializes
/// into the shared enum exactly as call_tool would.
#[test]
fn tagged_payloads_round_trip() {
    let get: GetCommand = serde_json::from_value(serde_json::json!({
        "entity": "issue",
        "id": 12345
    }))
    .expect("get payload deserializes");
    assert!(matches!(get, GetCommand::Issue(_)));

    let list: ListCommand = serde_json::from_value(serde_json::json!({
        "entity": "snippet_locations",
        "revision": "custom+org/repo$main",
        "with_lines": true
    }))
    .expect("list payload deserializes");
    assert!(matches!(list, ListCommand::SnippetLocations(_)));

    let update: UpdateCommand = serde_json::from_value(serde_json::json!({
        "entity": "project",
        "locator": "custom+org/repo",
        "title": "New Title"
    }))
    .expect("update payload deserializes");
    assert!(matches!(update, UpdateCommand::Project(_)));
}
