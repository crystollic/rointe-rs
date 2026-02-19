use std::time::Duration;

use reqwest::Client;
use serde::{de::DeserializeOwned, Serialize};
use tracing::debug;

use crate::error::{Result, RointeError};

// Firebase Realtime Database URL for the Rointe elife-prod project.
const DATABASE_URL: &str = "https://elife-prod.firebaseio.com";

// Per-request HTTP timeout. Firebase RTDB responses are typically fast;
// 30 seconds is generous enough to survive occasional latency spikes.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Thin wrapper around `reqwest::Client` for Firebase RTDB REST calls.
///
/// All methods accept `extra_params` for Firebase query filters such as
/// `orderBy` / `equalTo`. Pass `&[]` when no extra params are needed.
/// The `auth` token is always injected automatically.
pub struct RtdbClient {
    client: Client,
    base_url: String,
}

impl RtdbClient {
    /// Create a new `RtdbClient` using the provided `reqwest::Client`.
    ///
    /// The client should be configured with connection pooling and keepalive
    /// for production use — [`crate::RointeClient::new`] does this automatically.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            base_url: DATABASE_URL.to_string(),
        }
    }

    /// GET a JSON endpoint and deserialize into `T`.
    ///
    /// `extra_params` is a slice of `(key, value)` pairs appended to the
    /// query string after `auth=…`. Values are URL-encoded by reqwest.
    pub async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        token: &str,
        extra_params: &[(&str, &str)],
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        debug!("GET {url}");

        let mut all_params: Vec<(&str, &str)> = vec![("auth", token)];
        all_params.extend_from_slice(extra_params);

        let resp = self
            .client
            .get(&url)
            .query(&all_params)
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(RointeError::Network)?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(RointeError::Firebase(format!(
                "GET {path} failed ({status}): {body}"
            )));
        }

        resp.json().await.map_err(RointeError::Network)
    }

    /// PATCH a JSON endpoint with `body`.
    ///
    /// Used for all device control operations. The Firebase RTDB PATCH
    /// semantics perform a shallow merge — only the provided keys are updated.
    pub async fn patch<T: Serialize>(
        &self,
        path: &str,
        token: &str,
        body: &T,
    ) -> Result<()> {
        let url = format!("{}{}", self.base_url, path);
        debug!("PATCH {url}");

        let resp = self
            .client
            .patch(&url)
            .query(&[("auth", token)])
            .json(body)
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(RointeError::Network)?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(RointeError::Firebase(format!(
                "PATCH {path} failed ({status}): {body}"
            )));
        }

        Ok(())
    }

    /// Return the base URL this client is configured to use.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}
