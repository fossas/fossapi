//! FOSSA API CLI binary.
//!
//! A command-line interface for interacting with the FOSSA API.

use clap::Parser;
use fossapi::cli::{Cli, Command, Entity};
use fossapi::{
    get_dependencies, FossaClient, Get, Issue, List, Page, Project, ProjectUpdateParams, Revision,
    Update,
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
        Command::Get { entity, locator } => handle_get(client, entity, &locator, cli.json).await,
        Command::List {
            entity,
            page,
            count,
            revision,
        } => handle_list(client, entity, page, count, revision.as_deref(), cli.json).await,
        Command::Update {
            entity,
            locator,
            title,
            description,
        } => handle_update(client, entity, &locator, title, description, cli.json).await,
    }
}

async fn handle_get(
    client: &FossaClient,
    entity: Entity,
    locator: &str,
    json: bool,
) -> fossapi::Result<()> {
    match entity {
        Entity::Project => {
            let project = Project::get(client, locator.to_string()).await?;
            output_single(&project, json)?;
        }
        Entity::Revision => {
            let revision = Revision::get(client, locator.to_string()).await?;
            output_single(&revision, json)?;
        }
        Entity::Issue => {
            let id: u64 = locator
                .parse()
                .map_err(|_| fossapi::FossaError::InvalidLocator(locator.to_string()))?;
            let issue = Issue::get(client, id).await?;
            output_single(&issue, json)?;
        }
        Entity::Dependency => {
            eprintln!("Error: Dependencies can only be listed, not retrieved individually");
            eprintln!("Hint: Use 'fossapi list dependencies --revision <locator>'");
            return Err(fossapi::FossaError::InvalidLocator(
                "get dependency not supported".to_string(),
            ));
        }
    }
    Ok(())
}

async fn handle_list(
    client: &FossaClient,
    entity: Entity,
    page: Option<u32>,
    count: Option<u32>,
    revision: Option<&str>,
    json: bool,
) -> fossapi::Result<()> {
    let page = page.unwrap_or(1);
    let count = count.unwrap_or(20);

    match entity {
        Entity::Project => {
            let projects = Project::list_page(client, &Default::default(), page, count).await?;
            output_page(&projects, json, |p| ProjectRow::from(p))?;
        }
        Entity::Revision => {
            eprintln!("Error: Revisions must be listed for a specific project");
            eprintln!("Hint: Use 'fossapi get project <locator>' then 'fossapi list revisions --project <locator>'");
            return Err(fossapi::FossaError::InvalidLocator(
                "project locator required for revisions".to_string(),
            ));
        }
        Entity::Dependency => {
            let rev = revision.ok_or_else(|| {
                fossapi::FossaError::InvalidLocator(
                    "--revision required for listing dependencies".to_string(),
                )
            })?;
            let deps = get_dependencies(client, rev, Default::default()).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&deps)?);
            } else {
                let rows: Vec<DependencyRow> = deps.iter().map(DependencyRow::from).collect();
                println!("{}", Table::new(rows));
            }
        }
        Entity::Issue => {
            let issues = Issue::list_page(client, &Default::default(), page, count).await?;
            output_page(&issues, json, |i| IssueRow::from(i))?;
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
    json: bool,
) -> fossapi::Result<()> {
    match entity {
        Entity::Project => {
            let params = ProjectUpdateParams {
                title,
                description,
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

fn output_single<T: Serialize>(item: &T, _json: bool) -> fossapi::Result<()> {
    // TODO: Add table output for single items when json=false
    println!("{}", serde_json::to_string_pretty(item)?);
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
