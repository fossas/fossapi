//! FOSSA API CLI binary.
//!
//! A command-line interface for interacting with the FOSSA API.

use clap::Parser;
use fossapi::cli::{Cli, Command, Entity, GetCommand, ListCommand};
use fossapi::{
    get_dependencies, FossaClient, Get, Issue, List, Page, PrettyPrint, Project,
    ProjectUpdateParams, Revision, Update,
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
