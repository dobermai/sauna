use std::sync::Arc;

use reqwest::cookie::{CookieStore, Jar};
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use tracing::{debug, info, instrument, warn};

use crate::debug::{DebugConfig, HttpDebugger, Timer};
use crate::error::{KlafsError, Result};
use crate::models::{
    ConfigChangeRequest, GetSaunaStatusRequest, PowerControlRequest, SaunaInfo, SaunaMode,
    SaunaStatus, SetHumidityRequest, SetModeRequest, SetTemperatureRequest,
};

/// Default base URL for the Klafs API
pub const DEFAULT_BASE_URL: &str = "https://sauna-app.klafs.com";

/// User agent to use for requests (mimics the mobile app)
const USER_AGENT: &str = "KlafsSaunaApp/1.0";

/// Configuration for the Klafs client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Base URL for the API (can be overridden for testing)
    pub base_url: String,
    /// Debug configuration
    pub debug: DebugConfig,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            debug: DebugConfig::default(),
            timeout_secs: 30,
        }
    }
}

impl ClientConfig {
    /// Create a config for testing with a custom base URL
    pub fn for_testing(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            debug: DebugConfig::enabled(),
            timeout_secs: 5,
        }
    }
}

/// Klafs API client
///
/// Handles authentication and maintains session state for communicating
/// with the Klafs sauna control API.
///
/// # Example
///
/// ```no_run
/// use klafs_core::KlafsClient;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = KlafsClient::new();
///     client.login("user@example.com", "password").await?;
///
///     let status = client.get_status("sauna-uuid").await?;
///     println!("Temperature: {}°C", status.current_temperature);
///     Ok(())
/// }
/// ```
pub struct KlafsClient {
    client: Client,
    cookie_jar: Arc<Jar>,
    base_url: String,
    verification_token: std::sync::RwLock<Option<String>>,
    debugger: Arc<HttpDebugger>,
}

impl std::fmt::Debug for KlafsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KlafsClient")
            .field("base_url", &self.base_url)
            .field("is_logged_in", &self.is_logged_in())
            .finish()
    }
}

impl Default for KlafsClient {
    fn default() -> Self {
        Self::new()
    }
}

impl KlafsClient {
    /// Create a new Klafs client with default configuration
    pub fn new() -> Self {
        Self::with_config(ClientConfig::default())
    }

    /// Create a new Klafs client with custom configuration
    pub fn with_config(config: ClientConfig) -> Self {
        let cookie_jar = Arc::new(Jar::default());

        let client = Client::builder()
            .cookie_provider(cookie_jar.clone())
            .user_agent(USER_AGENT)
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            cookie_jar,
            base_url: config.base_url,
            verification_token: std::sync::RwLock::new(None),
            debugger: Arc::new(HttpDebugger::new(config.debug)),
        }
    }

    /// Get the debugger for accessing traffic logs
    pub fn debugger(&self) -> &HttpDebugger {
        &self.debugger
    }

    /// Enable debug logging at runtime
    pub fn enable_debug(&self) {
        self.debugger.enable();
    }

    /// Disable debug logging at runtime
    pub fn disable_debug(&self) {
        self.debugger.disable();
    }

    /// Login to the Klafs API
    ///
    /// This authenticates with the Klafs server and establishes a session.
    /// The session cookie is stored automatically for subsequent requests.
    ///
    /// # Arguments
    ///
    /// * `username` - Email address for the Klafs account
    /// * `password` - Password for the Klafs account
    ///
    /// # Errors
    ///
    /// Returns `KlafsError::AuthenticationFailed` if credentials are invalid.
    /// Returns `KlafsError::AccountLocked` if too many failed attempts.
    ///
    /// # Warning
    ///
    /// Klafs locks accounts after 3 failed login attempts!
    #[instrument(skip(self, password), fields(username = %username))]
    pub async fn login(&self, username: &str, password: &str) -> Result<()> {
        info!("Logging in as {}", username);
        let timer = Timer::start();

        // First, get the login page to extract any CSRF tokens
        let login_page_url = format!("{}/Account/Login", self.base_url);

        let request_id = self
            .debugger
            .log_request("GET", &login_page_url, &reqwest::header::HeaderMap::new(), None)
            .await;

        let login_page_response = self.client.get(&login_page_url).send().await?;

        let status_code = login_page_response.status();
        let headers = login_page_response.headers().clone();
        let login_page_html = login_page_response.text().await?;

        self.debugger
            .log_response(&request_id, status_code.as_u16(), &headers, Some(&login_page_html), timer.elapsed_ms())
            .await;

        if !status_code.is_success() {
            return Err(KlafsError::ApiError {
                status_code: status_code.as_u16(),
                message: "Failed to load login page".to_string(),
            });
        }

        // Extract the verification token from the login form
        let token = self.extract_verification_token(&login_page_html)?;
        debug!("Extracted verification token");

        // Submit the login form
        let login_url = format!("{}/Account/Login", self.base_url);

        let form_body = format!(
            "UserName={}&Password={}&__RequestVerificationToken={}",
            urlencoding::encode(username),
            urlencoding::encode(password),
            urlencoding::encode(&token)
        );

        let timer = Timer::start();
        let request_id = self
            .debugger
            .log_request("POST", &login_url, &reqwest::header::HeaderMap::new(), Some(&form_body))
            .await;

        let form_data = [
            ("UserName", username),
            ("Password", password),
            ("__RequestVerificationToken", &token),
        ];

        let response = self
            .client
            .post(&login_url)
            .form(&form_data)
            .send()
            .await?;

        let status = response.status();
        let headers = response.headers().clone();
        let response_text = response.text().await?;

        self.debugger
            .log_response(&request_id, status.as_u16(), &headers, Some(&response_text), timer.elapsed_ms())
            .await;

        // Check for error indicators in response
        if response_text.contains("Sicherheitskontrolle")
            || response_text.contains("security check")
        {
            warn!("Account may be locked due to security check");
            return Err(KlafsError::AccountLocked);
        }

        if response_text.contains("Invalid login") || response_text.contains("Ungültige Anmeldung")
        {
            return Err(KlafsError::AuthenticationFailed {
                message: "Invalid username or password".to_string(),
            });
        }

        // Check if we got redirected to the dashboard (successful login)
        // or stayed on the login page (failed login)
        if status.is_success() || status.is_redirection() {
            // Try to extract a new verification token for subsequent requests
            if let Ok(new_token) = self.extract_verification_token(&response_text) {
                let mut token_guard = self.verification_token.write().unwrap();
                *token_guard = Some(new_token);
            }

            // Verify we actually got a session cookie
            let base_url: Url = self.base_url.parse().expect("Invalid base URL");
            let cookies: Vec<_> = self.cookie_jar.cookies(&base_url).into_iter().collect();

            if cookies.is_empty() {
                return Err(KlafsError::AuthenticationFailed {
                    message: "No session cookie received".to_string(),
                });
            }

            info!("Login successful");
            Ok(())
        } else {
            Err(KlafsError::ApiError {
                status_code: status.as_u16(),
                message: format!("Login failed with status {}", status),
            })
        }
    }

    /// Get the current status of a sauna
    #[instrument(skip(self), fields(sauna_id = %sauna_id))]
    pub async fn get_status(&self, sauna_id: &str) -> Result<SaunaStatus> {
        debug!("Getting status for sauna {}", sauna_id);
        let timer = Timer::start();

        let url = format!("{}/Control/GetSaunaStatus", self.base_url);

        let request = GetSaunaStatusRequest {
            sauna_id: sauna_id.to_string(),
        };

        let body = serde_json::to_string(&request)?;
        let request_id = self
            .debugger
            .log_request("POST", &url, &reqwest::header::HeaderMap::new(), Some(&body))
            .await;

        let response = self.client.post(&url).json(&request).send().await?;

        let status = response.status();
        let headers = response.headers().clone();

        if status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::FORBIDDEN
        {
            return Err(KlafsError::SessionExpired);
        }

        let response_text = response.text().await?;

        self.debugger
            .log_response(&request_id, status.as_u16(), &headers, Some(&response_text), timer.elapsed_ms())
            .await;

        if !status.is_success() {
            return Err(KlafsError::ApiError {
                status_code: status.as_u16(),
                message: response_text,
            });
        }

        let sauna_status: SaunaStatus = serde_json::from_str(&response_text)?;

        debug!(
            "Sauna {} status: connected={}, powered={}",
            sauna_id, sauna_status.is_connected, sauna_status.is_powered_on
        );

        Ok(sauna_status)
    }

    /// List all saunas registered to the account
    #[instrument(skip(self))]
    pub async fn list_saunas(&self) -> Result<Vec<SaunaInfo>> {
        debug!("Fetching list of saunas");
        let timer = Timer::start();

        let url = format!("{}/SaunaApp/ChangeSettings", self.base_url);

        let request_id = self
            .debugger
            .log_request("GET", &url, &reqwest::header::HeaderMap::new(), None)
            .await;

        let response = self.client.get(&url).send().await?;

        let status = response.status();
        let headers = response.headers().clone();

        if status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::FORBIDDEN
        {
            return Err(KlafsError::SessionExpired);
        }

        let html = response.text().await?;

        self.debugger
            .log_response(&request_id, status.as_u16(), &headers, Some(&html), timer.elapsed_ms())
            .await;

        if !status.is_success() {
            return Err(KlafsError::ApiError {
                status_code: status.as_u16(),
                message: html,
            });
        }

        let saunas = self.extract_saunas_from_html(&html)?;

        info!("Found {} sauna(s)", saunas.len());

        Ok(saunas)
    }

    /// Power on the sauna
    ///
    /// # Arguments
    ///
    /// * `sauna_id` - UUID of the sauna
    /// * `pin` - PIN code for power control
    #[instrument(skip(self, pin), fields(sauna_id = %sauna_id))]
    pub async fn power_on(&self, sauna_id: &str, pin: &str) -> Result<()> {
        info!("Powering on sauna {}", sauna_id);

        self.send_power_command(sauna_id, pin, true).await
    }

    /// Power off the sauna
    ///
    /// # Arguments
    ///
    /// * `sauna_id` - UUID of the sauna
    /// * `pin` - PIN code for power control
    #[instrument(skip(self, pin), fields(sauna_id = %sauna_id))]
    pub async fn power_off(&self, sauna_id: &str, pin: &str) -> Result<()> {
        info!("Powering off sauna {}", sauna_id);

        self.send_power_command(sauna_id, pin, false).await
    }

    /// Internal helper for power commands
    async fn send_power_command(&self, sauna_id: &str, pin: &str, power_on: bool) -> Result<()> {
        let timer = Timer::start();

        let endpoint = if power_on { "StartCabin" } else { "StopCabin" };
        let url = format!("{}/SaunaApp/{}", self.base_url, endpoint);

        let request = PowerControlRequest {
            sauna_id: sauna_id.to_string(),
            pin: pin.to_string(),
        };

        let body = serde_json::to_string(&request)?;
        let request_id = self
            .debugger
            .log_request("POST", &url, &reqwest::header::HeaderMap::new(), Some(&body))
            .await;

        let response = self.client.post(&url).json(&request).send().await?;

        let status = response.status();
        let headers = response.headers().clone();
        let response_text = response.text().await?;

        self.debugger
            .log_response(&request_id, status.as_u16(), &headers, Some(&response_text), timer.elapsed_ms())
            .await;

        if status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::FORBIDDEN
        {
            return Err(KlafsError::SessionExpired);
        }

        // Check for PIN errors
        if response_text.contains("PIN") && response_text.contains("invalid") {
            return Err(KlafsError::InvalidPin);
        }

        if !status.is_success() {
            return Err(KlafsError::ApiError {
                status_code: status.as_u16(),
                message: response_text,
            });
        }

        info!(
            "Sauna {} power {}",
            sauna_id,
            if power_on { "on" } else { "off" }
        );

        Ok(())
    }

    /// Set the operating mode
    ///
    /// # Arguments
    ///
    /// * `sauna_id` - UUID of the sauna
    /// * `mode` - The mode to set (Sauna, Sanarium, or Infrared)
    #[instrument(skip(self), fields(sauna_id = %sauna_id, mode = ?mode))]
    pub async fn set_mode(&self, sauna_id: &str, mode: SaunaMode) -> Result<()> {
        info!("Setting mode to {:?} for sauna {}", mode, sauna_id);
        let timer = Timer::start();

        let url = format!("{}/SaunaApp/SetMode", self.base_url);

        let request = SetModeRequest {
            sauna_id: sauna_id.to_string(),
            mode: mode.into(),
        };

        let body = serde_json::to_string(&request)?;
        let request_id = self
            .debugger
            .log_request("POST", &url, &reqwest::header::HeaderMap::new(), Some(&body))
            .await;

        let response = self.client.post(&url).json(&request).send().await?;

        let status = response.status();
        let headers = response.headers().clone();
        let response_text = response.text().await?;

        self.debugger
            .log_response(&request_id, status.as_u16(), &headers, Some(&response_text), timer.elapsed_ms())
            .await;

        self.check_response_status(status, &response_text)?;

        info!("Mode set to {:?}", mode);
        Ok(())
    }

    /// Set the target temperature
    ///
    /// # Arguments
    ///
    /// * `sauna_id` - UUID of the sauna
    /// * `temperature` - Target temperature in °C
    ///   - Sauna mode: 10-100°C
    ///   - Sanarium mode: 40-75°C
    #[instrument(skip(self), fields(sauna_id = %sauna_id, temperature = %temperature))]
    pub async fn set_temperature(&self, sauna_id: &str, temperature: i32) -> Result<()> {
        // Validate temperature range
        if !(10..=100).contains(&temperature) {
            return Err(KlafsError::InvalidParameter {
                message: format!(
                    "Temperature must be between 10 and 100°C, got {}",
                    temperature
                ),
            });
        }

        info!(
            "Setting temperature to {}°C for sauna {}",
            temperature, sauna_id
        );
        let timer = Timer::start();

        let url = format!("{}/SaunaApp/ChangeTemperature", self.base_url);

        let request = SetTemperatureRequest {
            sauna_id: sauna_id.to_string(),
            temperature,
        };

        let body = serde_json::to_string(&request)?;
        let request_id = self
            .debugger
            .log_request("POST", &url, &reqwest::header::HeaderMap::new(), Some(&body))
            .await;

        let response = self.client.post(&url).json(&request).send().await?;

        let status = response.status();
        let headers = response.headers().clone();
        let response_text = response.text().await?;

        self.debugger
            .log_response(&request_id, status.as_u16(), &headers, Some(&response_text), timer.elapsed_ms())
            .await;

        self.check_response_status(status, &response_text)?;

        info!("Temperature set to {}°C", temperature);
        Ok(())
    }

    /// Set the humidity level (Sanarium mode only)
    ///
    /// # Arguments
    ///
    /// * `sauna_id` - UUID of the sauna
    /// * `level` - Humidity level (1-10)
    #[instrument(skip(self), fields(sauna_id = %sauna_id, level = %level))]
    pub async fn set_humidity(&self, sauna_id: &str, level: i32) -> Result<()> {
        // Validate humidity level
        if !(1..=10).contains(&level) {
            return Err(KlafsError::InvalidParameter {
                message: format!("Humidity level must be between 1 and 10, got {}", level),
            });
        }

        info!(
            "Setting humidity level to {} for sauna {}",
            level, sauna_id
        );
        let timer = Timer::start();

        let url = format!("{}/SaunaApp/ChangeHumLevel", self.base_url);

        let request = SetHumidityRequest {
            sauna_id: sauna_id.to_string(),
            hum_level: level,
        };

        let body = serde_json::to_string(&request)?;
        let request_id = self
            .debugger
            .log_request("POST", &url, &reqwest::header::HeaderMap::new(), Some(&body))
            .await;

        let response = self.client.post(&url).json(&request).send().await?;

        let status = response.status();
        let headers = response.headers().clone();
        let response_text = response.text().await?;

        self.debugger
            .log_response(&request_id, status.as_u16(), &headers, Some(&response_text), timer.elapsed_ms())
            .await;

        self.check_response_status(status, &response_text)?;

        info!("Humidity level set to {}", level);
        Ok(())
    }

    /// Set the scheduled start time
    ///
    /// # Arguments
    ///
    /// * `sauna_id` - UUID of the sauna
    /// * `hour` - Start hour (0-23)
    /// * `minute` - Start minute (0-59)
    #[instrument(skip(self), fields(sauna_id = %sauna_id, hour = %hour, minute = %minute))]
    pub async fn set_start_time(&self, sauna_id: &str, hour: i32, minute: i32) -> Result<()> {
        // Validate time
        if !(0..=23).contains(&hour) {
            return Err(KlafsError::InvalidParameter {
                message: format!("Hour must be between 0 and 23, got {}", hour),
            });
        }
        if !(0..=59).contains(&minute) {
            return Err(KlafsError::InvalidParameter {
                message: format!("Minute must be between 0 and 59, got {}", minute),
            });
        }

        info!(
            "Setting start time to {:02}:{:02} for sauna {}",
            hour, minute, sauna_id
        );
        let timer = Timer::start();

        let url = format!("{}/Control/PostConfigChange", self.base_url);

        let request = ConfigChangeRequest {
            sauna_id: sauna_id.to_string(),
            selected_sauna_temperature: None,
            selected_sanarium_temperature: None,
            selected_hum_level: None,
            selected_hour: Some(hour),
            selected_minute: Some(minute),
        };

        let body = serde_json::to_string(&request)?;
        let request_id = self
            .debugger
            .log_request("POST", &url, &reqwest::header::HeaderMap::new(), Some(&body))
            .await;

        let response = self.client.post(&url).json(&request).send().await?;

        let status = response.status();
        let headers = response.headers().clone();
        let response_text = response.text().await?;

        self.debugger
            .log_response(&request_id, status.as_u16(), &headers, Some(&response_text), timer.elapsed_ms())
            .await;

        self.check_response_status(status, &response_text)?;

        info!("Start time set to {:02}:{:02}", hour, minute);
        Ok(())
    }

    /// Check if the client has an active session
    pub fn is_logged_in(&self) -> bool {
        let base_url: Url = self.base_url.parse().expect("Invalid base URL");
        self.cookie_jar.cookies(&base_url).is_some()
    }

    /// Helper to check response status and convert to errors
    fn check_response_status(
        &self,
        status: reqwest::StatusCode,
        response_text: &str,
    ) -> Result<()> {
        if status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::FORBIDDEN
        {
            return Err(KlafsError::SessionExpired);
        }

        if !status.is_success() {
            return Err(KlafsError::ApiError {
                status_code: status.as_u16(),
                message: response_text.to_string(),
            });
        }

        Ok(())
    }

    /// Extract sauna information from the ChangeSettings HTML page
    fn extract_saunas_from_html(&self, html: &str) -> Result<Vec<SaunaInfo>> {
        let document = Html::parse_document(html);
        let mut saunas = Vec::new();

        // Look for table rows with sauna data
        let row_selector = Selector::parse("tr.iw-sauna-webgrid-row-style").unwrap();

        for row in document.select(&row_selector) {
            let id = row
                .value()
                .attr("data-sauna-id")
                .or_else(|| row.value().attr("data-id"))
                .map(|s| s.to_string());

            let id = id.or_else(|| {
                let text = row.text().collect::<String>();
                Self::extract_guid(&text)
            });

            let id = id.or_else(|| {
                let input_selector = Selector::parse("input[type='hidden']").unwrap();
                row.select(&input_selector)
                    .find_map(|input| input.value().attr("value"))
                    .and_then(|v| {
                        if Self::is_guid(v) {
                            Some(v.to_string())
                        } else {
                            None
                        }
                    })
            });

            let name = Self::extract_sauna_name(&row);

            if let (Some(id), Some(name)) = (id, name) {
                debug!("Found sauna: {} ({})", name, id);
                saunas.push(SaunaInfo { id, name });
            }
        }

        // Fallback: try alternative selectors
        if saunas.is_empty() {
            let alt_selector = Selector::parse("[data-sauna-id], [data-saunaid]").unwrap();
            for element in document.select(&alt_selector) {
                let id = element
                    .value()
                    .attr("data-sauna-id")
                    .or_else(|| element.value().attr("data-saunaid"))
                    .map(|s| s.to_string());

                let name = element.text().collect::<String>().trim().to_string();
                let name = if name.is_empty() {
                    element.value().attr("title").map(|s| s.to_string())
                } else {
                    Some(name)
                };

                if let (Some(id), Some(name)) = (id, name) {
                    if !name.is_empty() {
                        saunas.push(SaunaInfo { id, name });
                    }
                }
            }
        }

        Ok(saunas)
    }

    fn extract_sauna_name(row: &scraper::ElementRef) -> Option<String> {
        let label_selectors = [
            "td.sauna-name",
            "td:first-child",
            ".sauna-label",
            "label",
            "span.name",
        ];

        for selector_str in label_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(element) = row.select(&selector).next() {
                    let text = element.text().collect::<String>();
                    let text = text.trim();
                    if !text.is_empty() && !Self::is_guid(text) {
                        return Some(text.to_string());
                    }
                }
            }
        }

        let all_text = row.text().collect::<Vec<_>>();
        for text in all_text {
            let text = text.trim();
            if !text.is_empty() && !Self::is_guid(text) && text.len() > 2 {
                return Some(text.to_string());
            }
        }

        None
    }

    fn is_guid(s: &str) -> bool {
        uuid::Uuid::parse_str(s.trim()).is_ok()
    }

    fn extract_guid(text: &str) -> Option<String> {
        let guid_pattern = regex_lite::Regex::new(
            r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
        )
        .ok()?;

        guid_pattern
            .find(text)
            .map(|m| m.as_str().to_lowercase())
    }

    fn extract_verification_token(&self, html: &str) -> Result<String> {
        let document = Html::parse_document(html);

        let input_selector =
            Selector::parse("input[name='__RequestVerificationToken']").unwrap();

        if let Some(element) = document.select(&input_selector).next() {
            if let Some(value) = element.value().attr("value") {
                return Ok(value.to_string());
            }
        }

        let meta_selector = Selector::parse("meta[name='__RequestVerificationToken']").unwrap();

        if let Some(element) = document.select(&meta_selector).next() {
            if let Some(value) = element.value().attr("content") {
                return Ok(value.to_string());
            }
        }

        Err(KlafsError::VerificationTokenNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = KlafsClient::new();
        assert!(!client.is_logged_in());
    }

    #[test]
    fn test_client_with_config() {
        let config = ClientConfig::for_testing("http://localhost:8080");
        let client = KlafsClient::with_config(config);
        assert!(!client.is_logged_in());
        assert!(client.debugger().is_enabled());
    }

    #[test]
    fn test_extract_verification_token() {
        let client = KlafsClient::new();

        let html = r#"
            <html>
                <body>
                    <form>
                        <input name="__RequestVerificationToken" type="hidden" value="test-token-123" />
                    </form>
                </body>
            </html>
        "#;

        let token = client.extract_verification_token(html).unwrap();
        assert_eq!(token, "test-token-123");
    }

    #[test]
    fn test_extract_verification_token_not_found() {
        let client = KlafsClient::new();
        let html = "<html><body>No token here</body></html>";

        let result = client.extract_verification_token(html);
        assert!(matches!(result, Err(KlafsError::VerificationTokenNotFound)));
    }

    #[test]
    fn test_is_guid() {
        assert!(KlafsClient::is_guid("364cc9db-86f1-49d1-86cd-f6ef9b20a490"));
        assert!(KlafsClient::is_guid("00000000-0000-0000-0000-000000000000"));
        assert!(KlafsClient::is_guid("AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE"));

        assert!(!KlafsClient::is_guid("not-a-guid"));
        assert!(!KlafsClient::is_guid("364cc9db-86f1-49d1-86cd"));
        assert!(!KlafsClient::is_guid("364cc9db-86f1-49d1-86cd-f6ef9b20a490-extra"));
        assert!(!KlafsClient::is_guid("364cc9db_86f1_49d1_86cd_f6ef9b20a490"));
    }

    #[test]
    fn test_extract_guid() {
        assert_eq!(
            KlafsClient::extract_guid("Sauna ID: 364cc9db-86f1-49d1-86cd-f6ef9b20a490"),
            Some("364cc9db-86f1-49d1-86cd-f6ef9b20a490".to_string())
        );

        assert_eq!(KlafsClient::extract_guid("No GUID here"), None);

        let text = "First: 11111111-1111-1111-1111-111111111111, Second: 22222222-2222-2222-2222-222222222222";
        assert_eq!(
            KlafsClient::extract_guid(text),
            Some("11111111-1111-1111-1111-111111111111".to_string())
        );
    }

    #[test]
    fn test_extract_saunas_from_html() {
        let client = KlafsClient::new();

        let html = r#"
            <html>
                <body>
                    <table>
                        <tr class="iw-sauna-webgrid-row-style" data-sauna-id="364cc9db-86f1-49d1-86cd-f6ef9b20a490">
                            <td class="sauna-name">My Sauna</td>
                        </tr>
                    </table>
                </body>
            </html>
        "#;

        let saunas = client.extract_saunas_from_html(html).unwrap();
        assert_eq!(saunas.len(), 1);
        assert_eq!(saunas[0].id, "364cc9db-86f1-49d1-86cd-f6ef9b20a490");
        assert_eq!(saunas[0].name, "My Sauna");
    }
}
