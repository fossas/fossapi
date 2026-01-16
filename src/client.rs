//! FOSSA API client.
//!
//! Low-level HTTP client that handles authentication and raw requests.
//! Higher-level operations are implemented via traits on entity types.

use std::env;
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Client, Response};
use serde::Serialize;
use url::Url;

use crate::error::{FossaError, Result};

const DEFAULT_API_URL: &str = "https://app.fossa.com/api";
const USER_AGENT: &str = concat!("fossapi/", env!("CARGO_PKG_VERSION"));

/// Low-level FOSSA API client.
///
/// Handles authentication and HTTP requests. Entity-specific operations
/// are implemented via the `Get`, `List`, and `Update` traits on model types.
///
/// This struct is cheaply cloneable; clones reference the same underlying
/// connection pool.
///
/// # Example
///
/// ```no_run
/// use fossapi::FossaClient;
///
/// # async fn example() -> fossapi::Result<()> {
/// // Create from environment variables
/// let client = FossaClient::from_env()?;
///
/// // Or configure manually
/// let client = FossaClient::new("your-api-key", "https://app.fossa.com/api")?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct FossaClient {
    http: Client,
    base_url: Arc<Url>,
    token: String,
}

impl std::fmt::Debug for FossaClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FossaClient")
            .field("base_url", &self.base_url.as_str())
            .finish_non_exhaustive()
    }
}

impl FossaClient {
    /// Create a client from environment variables.
    ///
    /// Uses `FOSSA_API_KEY` for authentication and optionally `FOSSA_API_URL`
    /// for the base URL (defaults to `https://app.fossa.com/api`).
    ///
    /// # Errors
    ///
    /// Returns an error if `FOSSA_API_KEY` is not set.
    pub fn from_env() -> Result<Self> {
        let token = env::var("FOSSA_API_KEY").map_err(|_| {
            FossaError::ConfigMissing("FOSSA_API_KEY environment variable not set".to_string())
        })?;

        let base_url =
            env::var("FOSSA_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string());

        Self::new(&token, &base_url)
    }

    /// Create a new client with the provided token and base URL.
    ///
    /// # Arguments
    ///
    /// * `token` - FOSSA API key
    /// * `base_url` - Base URL for the FOSSA API (e.g., `https://app.fossa.com/api`)
    ///
    /// # Errors
    ///
    /// Returns an error if the base URL is invalid.
    pub fn new(token: &str, base_url: &str) -> Result<Self> {
        // Ensure base URL ends with /
        let base_url_str = if base_url.ends_with('/') {
            base_url.to_string()
        } else {
            format!("{base_url}/")
        };

        let base_url = Url::parse(&base_url_str)?;

        let http = Client::builder()
            .user_agent(USER_AGENT)
            .brotli(true)
            .gzip(true)
            .deflate(true)
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(FossaError::HttpError)?;

        Ok(Self {
            http,
            base_url: Arc::new(base_url),
            token: token.to_string(),
        })
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// Make a GET request.
    #[tracing::instrument(skip(self))]
    pub async fn get(&self, path: &str) -> Result<Response> {
        let url = self.base_url.join(path)?;

        let response = self
            .http
            .get(url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(FossaError::HttpError)?;

        Self::check_response(response).await
    }

    /// Make a GET request with query parameters.
    #[tracing::instrument(skip(self, query))]
    pub async fn get_with_query<Q: Serialize + ?Sized>(
        &self,
        path: &str,
        query: &Q,
    ) -> Result<Response> {
        let url = self.base_url.join(path)?;

        let response = self
            .http
            .get(url)
            .bearer_auth(&self.token)
            .query(query)
            .send()
            .await
            .map_err(FossaError::HttpError)?;

        Self::check_response(response).await
    }

    /// Make a PUT request with JSON body.
    #[tracing::instrument(skip(self, body))]
    pub async fn put<B: Serialize + ?Sized>(&self, path: &str, body: &B) -> Result<Response> {
        let url = self.base_url.join(path)?;

        let response = self
            .http
            .put(url)
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .await
            .map_err(FossaError::HttpError)?;

        Self::check_response(response).await
    }

    /// Make a POST request with JSON body.
    #[tracing::instrument(skip(self, body))]
    pub async fn post<B: Serialize + ?Sized>(&self, path: &str, body: &B) -> Result<Response> {
        let url = self.base_url.join(path)?;

        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .await
            .map_err(FossaError::HttpError)?;

        Self::check_response(response).await
    }

    /// Check response status and convert errors.
    async fn check_response(response: Response) -> Result<Response> {
        let status = response.status();

        if status.is_success() {
            return Ok(response);
        }

        // Handle rate limiting
        if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok());
            return Err(FossaError::RateLimited {
                retry_after_secs: retry_after,
            });
        }

        let message = Self::extract_error_message(response, status).await;
        Err(FossaError::ApiError {
            message,
            status_code: Some(status.as_u16()),
        })
    }

    /// Extract error message from a failed response.
    async fn extract_error_message(
        response: Response,
        status: reqwest::StatusCode,
    ) -> String {
        let body = match response.text().await {
            Ok(b) => b,
            Err(_) => return format!("HTTP {status}"),
        };

        // Try to parse as JSON and extract message field
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(msg) = json.get("message").and_then(|m| m.as_str()) {
                return msg.to_string();
            }
            if let Some(err) = json.get("error").and_then(|m| m.as_str()) {
                return err.to_string();
            }
        }

        body
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_debug() {
        let client = FossaClient::new("test-token", "https://app.fossa.com/api").unwrap();
        let debug = format!("{:?}", client);
        assert!(debug.contains("FossaClient"));
        assert!(debug.contains("base_url"));
        // Token should not be in debug output
        assert!(!debug.contains("test-token"));
    }

    #[test]
    fn test_base_url_trailing_slash() {
        let client1 = FossaClient::new("token", "https://app.fossa.com/api").unwrap();
        let client2 = FossaClient::new("token", "https://app.fossa.com/api/").unwrap();
        assert_eq!(client1.base_url().as_str(), client2.base_url().as_str());
    }
}
