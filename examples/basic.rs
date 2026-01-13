//! Basic example demonstrating the FOSSA API client.
//!
//! Run with:
//! ```
//! FOSSA_API_KEY=your-key cargo run --example basic
//! ```

use fossa_api::{
    get_dependencies, DependencyListQuery, FossaClient, Get, List, Project, ProjectListQuery,
};

#[tokio::main]
async fn main() -> fossa_api::Result<()> {
    // Initialize tracing for debugging (optional)
    tracing_subscriber::fmt::init();

    // Create client from environment variables
    println!("Creating FOSSA client...");
    let client = FossaClient::from_env()?;
    println!("Connected to: {}", client.base_url());

    // List first page of projects
    println!("\n--- Listing Projects (first page) ---");
    let projects_page = Project::list_page(&client, &ProjectListQuery::default(), 1, 10).await?;
    println!(
        "Found {} projects (total: {:?})",
        projects_page.len(),
        projects_page.total
    );

    for project in &projects_page {
        println!("  - {} ({})", project.title, project.id);
    }

    // Get a specific project (using the first one from the list)
    if let Some(first_project) = projects_page.items.first() {
        println!("\n--- Getting Project Details ---");
        let project = Project::get(&client, first_project.id.clone()).await?;
        println!("Project: {}", project.title);
        println!("  ID: {}", project.id);
        println!("  Type: {}", project.project_type.as_deref().unwrap_or("unknown"));
        println!("  Public: {}", project.public);
        println!("  Issues: {:?}", project.issues);

        // If the project has a revision, list its dependencies
        if let Some(revision_locator) = project.latest_revision_locator() {
            println!("\n--- Listing Dependencies for {} ---", revision_locator);
            let deps =
                get_dependencies(&client, revision_locator, DependencyListQuery::default()).await?;
            println!("Found {} dependencies", deps.len());

            let direct: Vec<_> = deps.iter().filter(|d| d.is_direct()).collect();
            let transitive: Vec<_> = deps.iter().filter(|d| d.is_transitive()).collect();
            println!("  Direct: {}", direct.len());
            println!("  Transitive: {}", transitive.len());

            // Show first 5 direct dependencies
            println!("\nFirst 5 direct dependencies:");
            for dep in direct.iter().take(5) {
                let name = dep.package_name().unwrap_or(&dep.locator);
                let version = dep.version().unwrap_or("unknown");
                let issues = if dep.has_issues() {
                    format!(" ({} issues)", dep.issues.len())
                } else {
                    String::new()
                };
                println!("  - {}@{}{}", name, version, issues);
            }
        }
    }

    println!("\nDone!");
    Ok(())
}
