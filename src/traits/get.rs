//! Get trait for fetching single entities.

use async_trait::async_trait;

use crate::client::FossaClient;
use crate::error::Result;

/// Fetch a single entity by ID.
///
/// Implement this trait for entity types that can be fetched individually
/// by a unique identifier (typically a locator string).
///
/// # Example
///
/// ```ignore
/// use fossa_api::{FossaClient, Project, Get};
///
/// let client = FossaClient::from_env()?;
/// let project = Project::get(&client, "custom+org/project".to_string()).await?;
/// ```
#[async_trait]
pub trait Get: Sized {
    /// The ID type for this entity (e.g., String locator).
    type Id;

    /// Fetch the entity by ID.
    ///
    /// # Arguments
    ///
    /// * `client` - The FOSSA API client
    /// * `id` - The entity identifier
    ///
    /// # Errors
    ///
    /// Returns an error if the entity is not found or the request fails.
    async fn get(client: &FossaClient, id: Self::Id) -> Result<Self>;
}
