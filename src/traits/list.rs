//! List trait for fetching collections of entities.

use async_trait::async_trait;

use crate::client::FossaClient;
use crate::error::Result;
use crate::pagination::Page;

/// Default page size for list operations.
pub const DEFAULT_PAGE_SIZE: u32 = 100;

/// Maximum pages to fetch (safety limit).
const MAX_PAGES: u32 = 1000;

/// List/filter entities with pagination support.
///
/// Implement this trait for entity types that can be listed with
/// optional filtering and pagination.
///
/// # Example
///
/// ```ignore
/// use fossa_api::{FossaClient, Project, List};
///
/// let client = FossaClient::from_env()?;
///
/// // Fetch a single page
/// let page = Project::list_page(&client, &Default::default(), 1, 50).await?;
///
/// // Fetch all pages
/// let all_projects = Project::list_all(&client, &Default::default()).await?;
/// ```
#[async_trait]
pub trait List: Sized + Send {
    /// Query parameters for filtering.
    type Query: Default + Send + Sync;

    /// List entities matching the query (single page).
    ///
    /// # Arguments
    ///
    /// * `client` - The FOSSA API client
    /// * `query` - Query parameters for filtering
    /// * `page` - Page number (1-indexed)
    /// * `count` - Number of items per page (max 100)
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    async fn list_page(
        client: &FossaClient,
        query: &Self::Query,
        page: u32,
        count: u32,
    ) -> Result<Page<Self>>;

    /// List all entities matching the query (fetches all pages).
    ///
    /// This method automatically handles pagination, fetching pages
    /// until no more results are available.
    ///
    /// # Arguments
    ///
    /// * `client` - The FOSSA API client
    /// * `query` - Query parameters for filtering
    ///
    /// # Errors
    ///
    /// Returns an error if any page request fails.
    async fn list_all(client: &FossaClient, query: &Self::Query) -> Result<Vec<Self>> {
        let mut all_items = Vec::new();
        let mut page = 1;

        loop {
            let result = Self::list_page(client, query, page, DEFAULT_PAGE_SIZE).await?;
            let items_count = result.items.len();
            all_items.extend(result.items);

            if !result.has_more || items_count < DEFAULT_PAGE_SIZE as usize {
                break;
            }
            page += 1;

            // Safety limit to prevent infinite loops
            if page > MAX_PAGES {
                tracing::warn!(
                    "Reached pagination limit of {} pages, stopping",
                    MAX_PAGES
                );
                break;
            }
        }

        Ok(all_items)
    }
}
