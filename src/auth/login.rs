use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use std::error::Error;

const LOGIN_URL: &str = "https://public-ubiservices.ubi.com/v3/profiles/sessions";
const APP_ID: &str = "f68a4bb5-608a-4ff2-8123-be8ef797e0a6";
const USER_AGENT: &str = "Massgate";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LoginResponse {
    pub ticket: Option<String>,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub profile_id: Option<String>,
    pub name_on_platform: Option<String>,
    pub remember_me_ticket: Option<String>,
    pub two_factor_authentication_ticket: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginRequest {
    remember_me: bool,
}

#[derive(Debug, Clone)]
pub struct LoginCredentials {
    pub ticket: String,
    pub session_id: String,
}

pub fn login(email: &str, password: &str) -> Result<LoginCredentials, Box<dyn Error>> {
    let credentials = format!("{}:{}", email, password);
    let b64_credentials = BASE64.encode(credentials.as_bytes());

    log::info!("Attempting login for email: {}", email);

    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .build()?;

    let response = client
        .post(LOGIN_URL)
        .header("Authorization", format!("Basic {}", b64_credentials))
        .header("Ubi-AppId", APP_ID)
        .header("Ubi-RequestedPlatformType", "uplay")
        .header("Content-Type", "application/json")
        .json(&LoginRequest { remember_me: true })
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Login failed with status {}: {}", status, body).into());
    }

    let login_response: LoginResponse = response.json()?;

    if login_response.two_factor_authentication_ticket.is_some() && login_response.ticket.is_none()
    {
        return Err("Two-factor authentication is required but not supported yet".into());
    }

    let ticket = login_response
        .ticket
        .ok_or("Login response missing ticket")?;
    let session_id = login_response
        .session_id
        .ok_or("Login response missing session_id")?;

    log::info!(
        "Login successful for user: {:?}",
        login_response.name_on_platform
    );

    Ok(LoginCredentials { ticket, session_id })
}
