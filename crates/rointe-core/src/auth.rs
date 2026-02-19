use std::time::{Duration, Instant};

use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, info};

use crate::error::{Result, RointeError};

// Firebase Identity Toolkit API key for the Rointe Connect application.
//
// This key is intentionally public — it is embedded in the Rointe Connect web
// and mobile app binaries and is not a secret. It identifies the Firebase
// project (elife-prod) but does not grant any elevated privileges on its own;
// authentication still requires valid user credentials.
const API_KEY: &str = "AIzaSyBi1DFJlBr9Cezf2BwfaT-PRPYmi3X3pdA";

// Firebase Identity Toolkit endpoint for email/password login.
const VERIFY_PASSWORD_URL: &str =
    "https://www.googleapis.com/identitytoolkit/v3/relyingparty/verifyPassword";

// Google Secure Token Service endpoint for refreshing id tokens.
const REFRESH_TOKEN_URL: &str = "https://securetoken.googleapis.com/v1/token";

// Refresh the token this many seconds before it actually expires to avoid
// using a token that expires mid-request.
const TOKEN_REFRESH_BUFFER: Duration = Duration::from_secs(300);

#[derive(Debug, Deserialize)]
struct LoginResponse {
    #[serde(rename = "idToken")]
    id_token: String,
    #[serde(rename = "refreshToken")]
    refresh_token: String,
    #[serde(rename = "expiresIn")]
    expires_in: String,
    #[serde(rename = "localId")]
    local_id: String,
}

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    id_token: String,
    refresh_token: String,
    expires_in: String,
}

/// Holds Firebase auth tokens and handles transparent refresh.
///
/// Obtain an instance via [`FirebaseAuth::login`]. The short-lived `id_token`
/// is refreshed automatically when it is within 5 minutes of
/// expiry; only the long-lived `refresh_token` needs to be persisted across
/// restarts.
#[derive(Debug, Clone)]
pub struct FirebaseAuth {
    /// Current Firebase ID token (valid for ~1 hour).
    pub id_token: String,
    /// Long-lived refresh token used to obtain new ID tokens.
    pub refresh_token: String,
    /// Firebase user ID (`localId`) returned at login.
    pub local_id: String,
    expires_at: Instant,
    client: Client,
}

impl FirebaseAuth {
    /// Authenticate with email and password, returning a `FirebaseAuth` with
    /// fresh tokens.
    ///
    /// Uses the Firebase Identity Toolkit `verifyPassword` endpoint.
    pub async fn login(client: Client, email: &str, password: &str) -> Result<Self> {
        let url = format!("{}?key={}", VERIFY_PASSWORD_URL, API_KEY);
        let params = [
            ("email", email),
            ("password", password),
            ("returnSecureToken", "true"),
        ];

        let resp = client
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(RointeError::Network)?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(RointeError::Auth(format!("Login failed: {body}")));
        }

        let login: LoginResponse = resp.json().await.map_err(RointeError::Network)?;
        let expires_in: u64 = login.expires_in.parse().unwrap_or(3600);
        let expires_at = Instant::now() + Duration::from_secs(expires_in);

        info!("Authenticated as localId={}", login.local_id);

        Ok(Self {
            id_token: login.id_token,
            refresh_token: login.refresh_token,
            local_id: login.local_id,
            expires_at,
            client,
        })
    }

    /// Exchange the refresh token for a new id token.
    ///
    /// Called automatically by [`Self::ensure_valid_token`]; you rarely need
    /// to call this directly.
    pub async fn refresh(&mut self) -> Result<()> {
        let url = format!("{}?key={}", REFRESH_TOKEN_URL, API_KEY);
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", self.refresh_token.as_str()),
        ];

        let resp = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(RointeError::Network)?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(RointeError::Auth(format!("Token refresh failed: {body}")));
        }

        let refreshed: RefreshResponse = resp.json().await.map_err(RointeError::Network)?;
        let expires_in: u64 = refreshed.expires_in.parse().unwrap_or(3600);

        self.id_token = refreshed.id_token;
        self.refresh_token = refreshed.refresh_token;
        self.expires_at = Instant::now() + Duration::from_secs(expires_in);

        debug!("Token refreshed, valid for {expires_in}s");
        Ok(())
    }

    /// Return the current id token, refreshing it first if it is about to expire.
    ///
    /// This is the primary method used by [`crate::RointeClient`] before every
    /// Firebase request.
    pub async fn ensure_valid_token(&mut self) -> Result<String> {
        if Instant::now() + TOKEN_REFRESH_BUFFER >= self.expires_at {
            self.refresh().await?;
        }
        Ok(self.id_token.clone())
    }
}
