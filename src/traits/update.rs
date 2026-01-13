//! Update trait for modifying entities.

use async_trait::async_trait;

use crate::client::FossaClient;
use crate::error::Result;

/// Update an existing entity.
///
/// Implement this trait for entity types that can be modified
/// after creation.
///
/// # Example
///
/// ```ignore
/// use fossapi::{FossaClient, Project, Update, ProjectUpdateParams};
///
/// let client = FossaClient::from_env()?;
/// let updated = Project::update(
///     &client,
///     "custom+org/project".to_string(),
///     ProjectUpdateParams {
///         title: Some("New Title".to_string()),
///         ..Default::default()
///     },
/// ).await?;
/// ```
#[async_trait]
pub trait Update: Sized {
    /// The ID type for this entity.
    type Id;

    /// Parameters for the update.
    type Params;

    /// Update the entity and return the updated version.
    ///
    /// # Arguments
    ///
    /// * `client` - The FOSSA API client
    /// * `id` - The entity identifier
    /// * `params` - Update parameters
    ///
    /// # Errors
    ///
    /// Returns an error if the entity is not found or the request fails.
    async fn update(client: &FossaClient, id: Self::Id, params: Self::Params) -> Result<Self>;
}
