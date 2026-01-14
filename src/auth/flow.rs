use std::error::Error;

use super::{DemuxSocket, login};
use crate::config::UplayConfig;
use crate::services::{DenuvoConnection, OwnershipConnection};

/// Result of the authentication flow
#[derive(Debug, Clone)]
pub struct AuthResult {
    pub game_token: String,
    pub ownership_token: Option<String>,
}

pub fn authenticate_and_get_tokens(
    config: &UplayConfig,
    request_token: &str,
    dlcs: Vec<u32>,
) -> Result<AuthResult, Box<dyn Error>> {
    log::info!("Starting authentication flow for app: {}", config.app_id);

    let credentials = login(&config.email, &config.password)?;
    log::info!("HTTP login successful");

    let socket = DemuxSocket::connect()?;

    socket.push_version()?;

    if !socket.authenticate(&credentials.ticket, true)? {
        return Err("Demux authentication failed".into());
    }
    log::info!("Demux authentication successful");

    let mut ownership = OwnershipConnection::new(
        &socket,
        credentials.ticket.clone(),
        credentials.session_id.clone(),
    )?;
    let owned_games = ownership.get_owned_games()?;

    let our_app = owned_games.iter().find(|g| g.product_id == config.app_id);
    if our_app.is_none() {
        return Err(format!("You do not own app {} - cannot continue", config.app_id).into());
    }
    log::info!("Ownership verified for app: {}", config.app_id);

    let our_app = our_app.unwrap();
    let owned_dlcs: Vec<u32> = owned_games
        .iter()
        .filter(|g| {
            g.owned.unwrap_or(false) && our_app.product_associations.contains(&g.product_id)
        })
        .map(|g| g.product_id)
        .collect();
    log::info!("Found {} owned DLC associations", owned_dlcs.len());

    let (ownership_token_str, _expiration) = ownership.get_ownership_token(config.app_id)?;

    let mut denuvo = DenuvoConnection::new(&socket)?;
    let game_token = denuvo.get_game_token(&ownership_token_str, request_token)?;
    log::info!("Got game token");

    let ownership_list_token = if !dlcs.is_empty() || !owned_dlcs.is_empty() {
        let dlcs_to_validate = if dlcs.is_empty() { owned_dlcs } else { dlcs };
        match denuvo.get_ownership_list_token(config.app_id, &game_token, dlcs_to_validate) {
            Ok(token) => {
                log::info!("Got ownership list token");
                Some(token)
            }
            Err(e) => {
                log::warn!("Failed to get ownership list token: {}", e);
                None
            }
        }
    } else {
        None
    };

    socket.disconnect();

    Ok(AuthResult {
        game_token,
        ownership_token: ownership_list_token,
    })
}
