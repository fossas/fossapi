//! Basic example demonstrating the FOSSA API client.
//!
//! Run with:
//! ```
//! FOSSA_API_KEY=your-key cargo run --example basic
//! ```

use fossapi::{
    get_revisions, FossaClient, Get, List, Project, ProjectListQuery, RevisionListQuery,
};

#[tokio::main]
async fn main() -> fossapi::Result<()> {
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

        // List revisions for this project
        println!("\n--- Listing Revisions ---");
        let revisions = get_revisions(&client, &project.id, RevisionListQuery::default()).await?;
        println!("Found {} revisions", revisions.len());

        for (i, rev) in revisions.iter().take(5).enumerate() {
            let ref_name = rev.ref_from_locator().unwrap_or("unknown");
            let resolved = if rev.resolved { "resolved" } else { "pending" };
            let issues = rev.unresolved_issue_count.unwrap_or(0);
            println!("  {}. {} - {} ({} issues)", i + 1, ref_name, resolved, issues);
        }

        // Get the first revision and show its dependencies
        if let Some(first_rev) = revisions.first() {
            println!("\n--- Revision Details ---");
            println!("  Locator: {}", first_rev.locator);
            println!("  Resolved: {}", first_rev.resolved);
            println!("  Source: {:?}", first_rev.source);
            println!("  Issues: {}", first_rev.issue_count());

            // Get dependencies through the revision
            println!("\n--- Dependencies via Revision ---");
            let deps = first_rev.dependencies(&client).await?;
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
