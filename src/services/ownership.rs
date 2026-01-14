#![allow(deprecated)]

use prost::Message;
use std::error::Error;

use crate::auth::DemuxSocket;
use crate::proto::ownership::{
    Downstream, InitializeReq, OwnedGame, OwnershipTokenReq, Req, Upstream,
};

pub struct OwnershipConnection<'a> {
    socket: &'a DemuxSocket,
    connection_id: u32,
    ticket: String,
    session_id: String,
    request_id: u32,
}

impl<'a> OwnershipConnection<'a> {
    pub fn new(
        socket: &'a DemuxSocket,
        ticket: String,
        session_id: String,
    ) -> Result<Self, Box<dyn Error>> {
        let connection_id = socket.open_connection("ownership_service")?;

        Ok(Self {
            socket,
            connection_id,
            ticket,
            session_id,
            request_id: 1,
        })
    }

    fn next_request_id(&mut self) -> u32 {
        let id = self.request_id;
        self.request_id += 1;
        id
    }

    fn send_request(&mut self, req: Req) -> Result<Downstream, Box<dyn Error>> {
        let upstream = Upstream { request: Some(req) };

        let data = upstream.encode_to_vec();
        let response_data = self.socket.send_service_data(self.connection_id, &data)?;

        let downstream = Downstream::decode(response_data.as_slice())?;
        Ok(downstream)
    }

    pub fn get_owned_games(&mut self) -> Result<Vec<OwnedGame>, Box<dyn Error>> {
        log::info!("Initializing ownership service and getting owned games");

        let req = Req {
            request_id: self.next_request_id(),
            ubi_ticket: Some(self.ticket.clone()),
            ubi_session_id: Some(self.session_id.clone()),
            initialize_req: Some(InitializeReq {
                deprecated_test_config: None,
                get_associations: Some(true),
                proto_version: Some(7),
                branches: vec![],
                use_staging: Some(false),
                claims: vec![],
                client_ip_override: None,
                get_uplay_pc_ticket: None,
            }),
            register_ownership_req: None,
            register_ownership_by_cd_key_req: None,
            deprecated_get_product_from_cd_key_req: None,
            get_product_config_req: None,
            deprecated_get_latest_manifests_req: None,
            get_batch_download_urls_req: None,
            get_uplay_pc_ticket_req: None,
            retry_uplay_core_initialize_req: None,
            consume_ownership_req: None,
            switch_product_branch_req: None,
            unlock_product_branch_req: None,
            register_ownership_steam_pop_req: None,
            register_ownership_from_oculus_req: None,
            get_game_token_req: None,
            claim_keystorage_key_req: None,
            get_game_time_ticket_req: None,
            get_game_withdrawal_rights_req: None,
            waive_game_withdrawal_rights_req: None,
            sign_ownership_req: None,
            register_ownership_from_wegame_req: None,
            ownership_token_req: None,
            register_temporary_ownership_req: None,
        };

        let downstream = self.send_request(req)?;

        if let Some(rsp) = downstream.response {
            if let Some(init_rsp) = rsp.initialize_rsp {
                let games = init_rsp
                    .owned_games
                    .map(|og| og.owned_games)
                    .unwrap_or_default();

                log::info!("Found {} owned games", games.len());
                return Ok(games);
            }
        }

        Err("Unexpected response to initialize request".into())
    }

    pub fn get_ownership_token(
        &mut self,
        product_id: u32,
    ) -> Result<(String, u64), Box<dyn Error>> {
        log::info!("Requesting ownership token for product: {}", product_id);

        let req = Req {
            request_id: self.next_request_id(),
            ubi_ticket: Some(self.ticket.clone()),
            ubi_session_id: Some(self.session_id.clone()),
            initialize_req: None,
            register_ownership_req: None,
            register_ownership_by_cd_key_req: None,
            deprecated_get_product_from_cd_key_req: None,
            get_product_config_req: None,
            deprecated_get_latest_manifests_req: None,
            get_batch_download_urls_req: None,
            get_uplay_pc_ticket_req: None,
            retry_uplay_core_initialize_req: None,
            consume_ownership_req: None,
            switch_product_branch_req: None,
            unlock_product_branch_req: None,
            register_ownership_steam_pop_req: None,
            register_ownership_from_oculus_req: None,
            get_game_token_req: None,
            claim_keystorage_key_req: None,
            get_game_time_ticket_req: None,
            get_game_withdrawal_rights_req: None,
            waive_game_withdrawal_rights_req: None,
            sign_ownership_req: None,
            register_ownership_from_wegame_req: None,
            ownership_token_req: Some(OwnershipTokenReq {
                product_id: Some(product_id),
            }),
            register_temporary_ownership_req: None,
        };

        let downstream = self.send_request(req)?;

        if let Some(rsp) = downstream.response {
            if let Some(token_rsp) = rsp.ownership_token_rsp {
                let token = token_rsp.token.unwrap_or_default();
                let expiration = token_rsp.expiration.unwrap_or(0);
                log::info!("Got ownership token, expires at: {}", expiration);
                return Ok((token, expiration));
            }
        }

        Err("Unexpected response to ownership token request".into())
    }
}
