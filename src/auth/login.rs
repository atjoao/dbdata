use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use native_tls::TlsConnector;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

const LOGIN_HOST: &str = "public-ubiservices.ubi.com";
const LOGIN_PATH: &str = "/v3/profiles/sessions";
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

    let body = serde_json::to_string(&LoginRequest { remember_me: true })?;

    let request = format!(
        "POST {} HTTP/1.1\r\n\
         Host: {}\r\n\
         User-Agent: {}\r\n\
         Authorization: Basic {}\r\n\
         Ubi-AppId: {}\r\n\
         Ubi-RequestedPlatformType: uplay\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        LOGIN_PATH,
        LOGIN_HOST,
        USER_AGENT,
        b64_credentials,
        APP_ID,
        body.len(),
        body
    );

    log::info!("Connecting to {}:443...", LOGIN_HOST);
    let tcp_stream = TcpStream::connect((LOGIN_HOST, 443))?;
    tcp_stream.set_nodelay(true)?;

    let timeout = std::time::Duration::from_secs(30);
    tcp_stream.set_read_timeout(Some(timeout))?;
    tcp_stream.set_write_timeout(Some(timeout))?;

    log::info!("TCP connected, starting TLS handshake...");
    let connector = TlsConnector::new()?;
    let mut tls_stream = connector.connect(LOGIN_HOST, tcp_stream)?;
    log::info!("TLS handshake complete");

    log::info!("Sending HTTP request...");
    tls_stream.write_all(request.as_bytes())?;
    tls_stream.flush()?;
    log::info!("Request sent, reading response...");

    let mut reader = BufReader::new(tls_stream);

    let mut status_line = String::new();
    reader.read_line(&mut status_line)?;
    log::info!("Status: {}", status_line.trim());

    let status_code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let mut content_length: usize = 0;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header)?;
        let header = header.trim();
        if header.is_empty() {
            break;
        }
        if header.to_lowercase().starts_with("content-length:") {
            content_length = header
                .split(':')
                .nth(1)
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
        }
    }

    let mut body_bytes = vec![0u8; content_length];
    reader.read_exact(&mut body_bytes)?;
    let response_body = String::from_utf8_lossy(&body_bytes);

    if status_code < 200 || status_code >= 300 {
        return Err(format!(
            "Login failed with status {}: {}",
            status_code, response_body
        )
        .into());
    }

    log::info!("Parsing response...");
    let login_response: LoginResponse = serde_json::from_str(&response_body)?;

    if login_response.two_factor_authentication_ticket.is_some() && login_response.ticket.is_none()
    {
        return Err("Two-factor authentication is required for this account".into());
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
