//! FOSSA API CLI binary.
//!
//! A command-line interface for interacting with the FOSSA API. Argument
//! parsing and dispatch go through the shared operation declarations in
//! `fossapi::ops`, which the MCP server also consumes; this file only owns
//! presentation (tables and JSON printing).

use clap::Parser;
use fossapi::cli::{Cli, Command};
use fossapi::ops::{run_get, run_list, run_update, ListOutput};
use fossapi::{FossaClient, Page, PrettyPrint, Project, Snippet, SnippetLocation, SnippetPath};
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
        Command::Get { command } => {
            let output = run_get(client, command).await?;
            output_single(&output, cli.json)
        }
        Command::List { command } => {
            let output = run_list(client, command).await?;
            output_list(&output, cli.json)
        }
        Command::Update { command } => {
            let output = run_update(client, command).await?;
            output_single(&output, cli.json)
        }
        Command::Mcp { verbose } => handle_mcp(client, verbose).await,
    }
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
    let service = server
        .serve(transport)
        .await
        .map_err(|e| fossapi::FossaError::ConfigMissing(format!("MCP transport error: {e}")))?;

    service
        .waiting()
        .await
        .map_err(|e| fossapi::FossaError::ConfigMissing(format!("MCP service error: {e}")))?;

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

fn output_list(output: &ListOutput, json: bool) -> fossapi::Result<()> {
    match output {
        ListOutput::Projects(page) => output_page(page, json, |p| ProjectRow::from(p)),
        ListOutput::Issues(page) => output_page(page, json, |i| IssueRow::from(i)),
        ListOutput::Dependencies(page) => output_page(page, json, |d| DependencyRow::from(d)),
        ListOutput::Revisions(page) => output_page(page, json, |r| RevisionRow::from(r)),
        ListOutput::Snippets(page) => output_page(page, json, |s| SnippetRow::from(s)),
        // Page numbers/totals count the underlying snippets, not the rows.
        ListOutput::SnippetLocations(page) => {
            output_page(page, json, |l| SnippetLocationRow::from(l))
        }
        ListOutput::SnippetPaths(paths) => {
            if json {
                println!("{}", serde_json::to_string_pretty(paths)?);
            } else {
                let rows: Vec<SnippetPathRow> = paths.iter().map(SnippetPathRow::from).collect();
                println!("{}", Table::new(rows));
            }
            Ok(())
        }
    }
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
        let rows: Vec<R> = page.items.iter().map(&to_row).collect();
        println!("{}", Table::new(rows));
        if let Some(total) = page.total {
            let total_pages = total.div_ceil(page.count.max(1) as u64);
            println!(
                "\nPage {}/{} ({} total items)",
                page.page, total_pages, total
            );
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

impl From<&fossapi::Issue> for IssueRow {
    fn from(i: &fossapi::Issue) -> Self {
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
