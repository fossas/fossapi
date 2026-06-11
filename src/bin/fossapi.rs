//! FOSSA API CLI binary.
//!
//! A command-line interface for interacting with the FOSSA API.

use clap::Parser;
use fossapi::cli::{Cli, Command, Entity, GetCommand, ListCommand};
use fossapi::{
    get_dependencies, FossaClient, Get, Issue, List, Page, PrettyPrint, Project,
    ProjectUpdateParams, Revision, Snippet, SnippetListQuery, SnippetLocation, SnippetPath, Update,
};
use serde::Serialize;
use std::process::ExitCode;
use tabled::{Table, Tabled};

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let client = match FossaClient::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            eprintln!("Hint: Set FOSSA_API_KEY environment variable");
            return ExitCode::FAILURE;
        }
    };

    match run(&client, cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn run(client: &FossaClient, cli: Cli) -> fossapi::Result<()> {
    match cli.command {
        Command::Get { command } => handle_get(client, command, cli.json).await,
        Command::List { command } => handle_list(client, command, cli.json).await,
        Command::Update {
            entity,
            locator,
            title,
            description,
            public,
        } => handle_update(client, entity, &locator, title, description, public, cli.json).await,
        Command::Mcp { verbose } => handle_mcp(client, verbose).await,
    }
}

async fn handle_get(
    client: &FossaClient,
    command: GetCommand,
    json: bool,
) -> fossapi::Result<()> {
    match command {
        GetCommand::Project { locator } => {
            let project = Project::get(client, locator).await?;
            output_single(&project, json)?;
        }
        GetCommand::Revision { locator } => {
            let revision = Revision::get(client, locator).await?;
            output_single(&revision, json)?;
        }
        GetCommand::Issue { id } => {
            let issue = Issue::get(client, id).await?;
            output_single(&issue, json)?;
        }
        GetCommand::Snippet { revision, snippet } => {
            let details = fossapi::get_snippet_details(client, &revision, &snippet).await?;
            output_single(&details, json)?;
        }
        GetCommand::SnippetMatch {
            revision,
            snippet,
            path,
        } => {
            let details = fossapi::get_snippet_match(client, &revision, &snippet, &path).await?;
            output_single(&details, json)?;
        }
    }
    Ok(())
}

async fn handle_list(
    client: &FossaClient,
    command: ListCommand,
    json: bool,
) -> fossapi::Result<()> {
    match command {
        ListCommand::Projects { page, count } => {
            let page = page.unwrap_or(1);
            let count = count.unwrap_or(20);
            let projects = Project::list_page(client, &Default::default(), page, count).await?;
            output_page(&projects, json, |p| ProjectRow::from(p))?;
        }
        ListCommand::Issues { page, count } => {
            let page = page.unwrap_or(1);
            let count = count.unwrap_or(20);
            let issues = Issue::list_page(client, &Default::default(), page, count).await?;
            output_page(&issues, json, |i| IssueRow::from(i))?;
        }
        ListCommand::Dependencies { revision, revision_positional } => {
            let revision = revision.or(revision_positional).expect("revision is required");
            let deps = get_dependencies(client, &revision, Default::default()).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&deps)?);
            } else {
                let rows: Vec<DependencyRow> = deps.iter().map(DependencyRow::from).collect();
                println!("{}", Table::new(rows));
            }
        }
        ListCommand::Revisions {
            project,
            page,
            count,
        } => {
            let page = page.unwrap_or(1);
            let count = count.unwrap_or(20);
            let revisions =
                fossapi::get_revisions(client, &project, Default::default()).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&revisions)?);
            } else {
                let rows: Vec<RevisionRow> = revisions.iter().map(RevisionRow::from).collect();
                println!("{}", Table::new(rows));
                println!("\n{} revisions for {}", revisions.len(), project);
            }
            let _ = (page, count);
        }
        ListCommand::Snippets {
            revision,
            path,
            page,
            count,
        } => {
            let query = SnippetListQuery {
                path,
                ..Default::default()
            };
            let page = page.unwrap_or(1);
            let count = count.unwrap_or(20);
            let snippets = fossapi::get_snippets_page(client, &revision, query, page, count).await?;
            output_page(&snippets, json, |s| SnippetRow::from(s))?;
        }
        ListCommand::SnippetLocations {
            revision,
            path,
            with_lines,
        } => {
            let query = SnippetListQuery {
                path,
                ..Default::default()
            };
            let locations =
                fossapi::get_snippet_locations(client, &revision, query, with_lines).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&locations)?);
            } else {
                let rows: Vec<SnippetLocationRow> =
                    locations.iter().map(SnippetLocationRow::from).collect();
                println!("{}", Table::new(rows));
                println!("\n{} match location(s)", locations.len());
            }
        }
        ListCommand::SnippetPaths { revision, path } => {
            let query = SnippetListQuery {
                path,
                ..Default::default()
            };
            let paths = fossapi::get_snippet_paths(client, &revision, query).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&paths)?);
            } else {
                let rows: Vec<SnippetPathRow> = paths.iter().map(SnippetPathRow::from).collect();
                println!("{}", Table::new(rows));
            }
        }
    }
    Ok(())
}

async fn handle_update(
    client: &FossaClient,
    entity: Entity,
    locator: &str,
    title: Option<String>,
    description: Option<String>,
    public: Option<bool>,
    json: bool,
) -> fossapi::Result<()> {
    match entity {
        Entity::Project => {
            let params = ProjectUpdateParams {
                title,
                description,
                public,
                ..Default::default()
            };
            let project = Project::update(client, locator.to_string(), params).await?;
            output_single(&project, json)?;
        }
        _ => {
            eprintln!("Error: Only projects can be updated via CLI");
            return Err(fossapi::FossaError::InvalidLocator(
                "only projects support update".to_string(),
            ));
        }
    }
    Ok(())
}

async fn handle_mcp(client: &FossaClient, verbose: bool) -> fossapi::Result<()> {
    use fossapi::mcp::FossaServer;
    use rmcp::ServiceExt;

    if verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(std::io::stderr)
            .init();
    }

    let server = FossaServer::new(client.clone());
    let transport = rmcp::transport::stdio();
    let service = server.serve(transport).await.map_err(|e| {
        fossapi::FossaError::ConfigMissing(format!("MCP transport error: {e}"))
    })?;

    service.waiting().await.map_err(|e| {
        fossapi::FossaError::ConfigMissing(format!("MCP service error: {e}"))
    })?;

    Ok(())
}

fn output_single<T: Serialize + PrettyPrint>(item: &T, json: bool) -> fossapi::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(item)?);
    } else {
        println!("{}", item.pretty_print());
    }
    Ok(())
}

fn output_page<T, R, F>(page: &Page<T>, json: bool, to_row: F) -> fossapi::Result<()>
where
    T: Serialize,
    R: Tabled,
    F: Fn(&T) -> R,
{
    if json {
        println!("{}", serde_json::to_string_pretty(&page.items)?);
    } else {
        let rows: Vec<R> = page.items.iter().map(to_row).collect();
        println!("{}", Table::new(rows));
        if let Some(total) = page.total {
            let total_pages = (total + page.count as u64 - 1) / page.count.max(1) as u64;
            println!("\nPage {}/{} ({} total items)", page.page, total_pages, total);
        } else if page.has_more {
            println!("\nPage {} (more available)", page.page);
        } else {
            println!("\nPage {} (end)", page.page);
        }
    }
    Ok(())
}

// Table row types for non-JSON output

#[derive(Tabled)]
struct ProjectRow {
    locator: String,
    title: String,
    issues: String,
}

impl From<&Project> for ProjectRow {
    fn from(p: &Project) -> Self {
        Self {
            locator: p.locator().to_string(),
            title: p.title.clone(),
            issues: p
                .issues
                .as_ref()
                .map(|i| i.total.to_string())
                .unwrap_or_default(),
        }
    }
}

#[derive(Tabled)]
struct IssueRow {
    id: u64,
    #[tabled(rename = "type")]
    issue_type: String,
    severity: String,
    source: String,
}

impl From<&Issue> for IssueRow {
    fn from(i: &Issue) -> Self {
        Self {
            id: i.id,
            issue_type: i.issue_type.clone(),
            severity: i.severity.clone().unwrap_or_default(),
            source: i.source.name.clone().unwrap_or_else(|| i.source.id.clone()),
        }
    }
}

#[derive(Tabled)]
struct DependencyRow {
    locator: String,
    depth: String,
    licenses: String,
}

impl From<&fossapi::Dependency> for DependencyRow {
    fn from(d: &fossapi::Dependency) -> Self {
        Self {
            locator: d.locator.clone(),
            depth: if d.is_direct() {
                "direct".to_string()
            } else {
                format!("transitive ({})", d.depth)
            },
            licenses: d
                .licenses
                .iter()
                .filter_map(|l| l.id())
                .collect::<Vec<_>>()
                .join(", "),
        }
    }
}

#[derive(Tabled)]
struct RevisionRow {
    locator: String,
    resolved: String,
    source: String,
}

impl From<&fossapi::Revision> for RevisionRow {
    fn from(r: &fossapi::Revision) -> Self {
        Self {
            locator: r.locator.clone(),
            resolved: if r.resolved { "yes" } else { "no" }.to_string(),
            source: r.source.clone().unwrap_or_default(),
        }
    }
}

#[derive(Tabled)]
struct SnippetRow {
    id: String,
    package: String,
    version: String,
    #[tabled(rename = "match")]
    match_pct: String,
    files: u32,
}

impl From<&Snippet> for SnippetRow {
    fn from(s: &Snippet) -> Self {
        Self {
            id: s.id.clone(),
            package: s.package.clone(),
            version: s.version.clone(),
            match_pct: format!("{:.0}%", s.highest_match_percentage * 100.0),
            files: s.match_count,
        }
    }
}

#[derive(Tabled)]
struct SnippetLocationRow {
    file: String,
    lines: String,
    package: String,
    #[tabled(rename = "match")]
    match_pct: String,
    snippet: String,
}

impl From<&SnippetLocation> for SnippetLocationRow {
    fn from(l: &SnippetLocation) -> Self {
        let lines = match (l.line_start, l.line_end) {
            (Some(lo), Some(hi)) => format!("{lo}-{hi}"),
            _ => "-".to_string(),
        };
        Self {
            file: l.path.clone(),
            lines,
            package: format!("{} {}", l.package, l.version),
            match_pct: format!("{:.0}%", l.match_percentage * 100.0),
            snippet: l.snippet_id.clone(),
        }
    }
}

#[derive(Tabled)]
struct SnippetPathRow {
    #[tabled(rename = "type")]
    path_type: String,
    path: String,
    count: u32,
}

impl From<&SnippetPath> for SnippetPathRow {
    fn from(p: &SnippetPath) -> Self {
        Self {
            path_type: p.path_type.clone(),
            path: p.path.clone(),
            count: p.count,
        }
    }
}
