use std::error::Error;
use prost::Message;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

use crate::auth::DemuxSocket;
use crate::proto::denuvo::{
    Upstream, Downstream, Req, GetGameTokenReq, GetOwnershipListTokenReq,
    rsp::Result as DenuvoResult,
};

pub struct DenuvoConnection<'a> {
    socket: &'a DemuxSocket,
    connection_id: u32,
    request_id: u32,
}

impl<'a> DenuvoConnection<'a> {
    pub fn new(socket: &'a DemuxSocket) -> Result<Self, Box<dyn Error>> {
        let connection_id = socket.open_connection("denuvo_service")?;
        
        Ok(Self {
            socket,
            connection_id,
            request_id: 1,
        })
    }

    fn next_request_id(&mut self) -> u32 {
        let id = self.request_id;
        self.request_id += 1;
        id
    }

    fn send_request(&mut self, req: Req) -> Result<Downstream, Box<dyn Error>> {
        let upstream = Upstream {
            request: Some(req),
        };
        
        let data = upstream.encode_to_vec();
        let response_data = self.socket.send_service_data(self.connection_id, &data)?;
        
        let downstream = Downstream::decode(response_data.as_slice())?;
        Ok(downstream)
    }

    pub fn get_game_token(
        &mut self,
        ownership_token: &str,
        request_token: &str,
    ) -> Result<String, Box<dyn Error>> {
        log::info!("Requesting game token from denuvo service");
        
        let request_token_bytes = BASE64.encode(request_token.as_bytes());
        let request_token_bytes = BASE64.decode(&request_token_bytes)?;
        
        let req = Req {
            request_id: self.next_request_id(),
            get_game_token_req: Some(GetGameTokenReq {
                ownership_token: ownership_token.to_string(),
                request_token: request_token_bytes,
            }),
            get_game_time_token_req: None,
            get_ownership_list_token_req: None,
        };
        
        let downstream = self.send_request(req)?;
        
        if let Some(rsp) = downstream.response {
            if rsp.result != DenuvoResult::Success as i32 {
                return Err(format!("Denuvo request failed with result: {}", rsp.result).into());
            }
            
            if let Some(token_rsp) = rsp.get_game_token_rsp {
                let token = String::from_utf8(
                    BASE64.decode(BASE64.encode(&token_rsp.game_token))?
                ).map_err(|_| "Invalid UTF-8 in game token")?;
                
                log::info!("Got game token successfully");
                return Ok(token);
            }
        }
        
        Err("Unexpected response to get game token request".into())
    }

    pub fn get_ownership_list_token(
        &mut self,
        product_id: u32,
        game_token: &str,
        dlcs: Vec<u32>,
    ) -> Result<String, Box<dyn Error>> {
        log::info!("Requesting ownership list token for product: {} with {} DLCs", product_id, dlcs.len());
        
        let game_token_bytes = BASE64.encode(game_token.as_bytes());
        let game_token_bytes = BASE64.decode(&game_token_bytes)?;
        
        let req = Req {
            request_id: self.next_request_id(),
            get_game_token_req: None,
            get_game_time_token_req: None,
            get_ownership_list_token_req: Some(GetOwnershipListTokenReq {
                product_id,
                game_token: game_token_bytes,
                addons_to_validate: dlcs,
            }),
        };
        
        let downstream = self.send_request(req)?;
        
        if let Some(rsp) = downstream.response {
            if rsp.result != DenuvoResult::Success as i32 {
                return Err(format!("Denuvo ownership list request failed with result: {}", rsp.result).into());
            }
            
            if let Some(list_rsp) = rsp.get_ownership_list_token_rsp {
                let token = String::from_utf8(
                    BASE64.decode(BASE64.encode(&list_rsp.ownership_list_token))?
                ).map_err(|_| "Invalid UTF-8 in ownership list token")?;
                
                log::info!("Got ownership list token successfully");
                return Ok(token);
            }
        }
        
        Err("Unexpected response to get ownership list token request".into())
    }
}
