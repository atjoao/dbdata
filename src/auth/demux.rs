use native_tls::TlsConnector;
use prost::Message;
use std::net::TcpStream;
use std::{
    error::Error,
    io::{Read, Write},
    sync::Mutex,
};

use crate::proto::demux::{
    AuthenticateReq, ClientVersionPush, DataMessage, Downstream, GetPatchInfoReq,
    OpenConnectionReq, Push, Req, Token, Upstream,
};

const DEMUX_HOST: &str = "dmx.upc.ubisoft.com";
const DEMUX_PORT: u16 = 443;

pub struct DemuxSocket {
    stream: Mutex<native_tls::TlsStream<TcpStream>>,
    request_id: Mutex<u32>,
}

impl DemuxSocket {
    pub fn connect() -> Result<Self, Box<dyn Error>> {
        log::info!(
            "Connecting to demux server at {}:{}",
            DEMUX_HOST,
            DEMUX_PORT
        );

        let tcp_stream = TcpStream::connect((DEMUX_HOST, DEMUX_PORT))?;
        tcp_stream.set_nodelay(true)?;

        let timeout = std::time::Duration::from_secs(30);
        tcp_stream.set_read_timeout(Some(timeout))?;
        tcp_stream.set_write_timeout(Some(timeout))?;

        let connector = TlsConnector::new()?;
        let tls_stream = connector.connect(DEMUX_HOST, tcp_stream)?;

        log::info!("Connected to demux server");

        Ok(Self {
            stream: Mutex::new(tls_stream),
            request_id: Mutex::new(1),
        })
    }

    pub fn disconnect(&self) {
        log::info!("Disconnecting from demux server");
        if let Ok(mut stream) = self.stream.lock() {
            let _ = stream.shutdown();
        }
    }

    fn next_request_id(&self) -> u32 {
        let mut id = self.request_id.lock().unwrap();
        let current = *id;
        *id += 1;
        current
    }

    fn send_raw(&self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let mut stream = self.stream.lock().unwrap();

        let len = data.len() as u32;
        stream.write_all(&len.to_be_bytes())?;

        stream.write_all(data)?;
        stream.flush()?;

        Ok(())
    }

    fn recv_raw(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut stream = self.stream.lock().unwrap();

        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf)?;
        let len = u32::from_be_bytes(len_buf) as usize;

        if len == 0 {
            return Err("Received zero-length message".into());
        }
        if len > 1024 * 1024 {
            return Err(format!("Message length {} too large", len).into());
        }

        let mut data = vec![0u8; len];
        stream.read_exact(&mut data)?;

        Ok(data)
    }

    fn send_upstream_msg(&self, upstream: Upstream) -> Result<Downstream, Box<dyn Error>> {
        let data = upstream.encode_to_vec();
        self.send_raw(&data)?;

        let response_data = self.recv_raw()?;
        let downstream = Downstream::decode(response_data.as_slice())?;

        Ok(downstream)
    }

    pub fn push_version(&self) -> Result<(), Box<dyn Error>> {
        let latest_version = self.get_latest_version()?;
        log::info!("Pushing client version: {}", latest_version);

        let upstream = Upstream {
            request: None,
            push: Some(Push {
                data: None,
                connection_closed: None,
                keep_alive: None,
                client_version: Some(ClientVersionPush {
                    version: latest_version,
                }),
                client_outdated: None,
                product_started: None,
                product_ended: None,
            }),
        };

        let data = upstream.encode_to_vec();
        self.send_raw(&data)?;
        Ok(())
    }

    pub fn get_latest_version(&self) -> Result<u32, Box<dyn Error>> {
        log::info!("Getting latest version from server");

        let req = Req {
            request_id: self.next_request_id(),
            authenticate_req: None,
            get_patch_info_req: Some(GetPatchInfoReq {
                patch_track_id: "DEFAULT".to_string(),
                test_config: false,
                track_type: Some(0),
            }),
            service_request: None,
            open_connection_req: None,
            client_ip_override: None,
        };

        let upstream = Upstream {
            request: Some(req),
            push: None,
        };

        let downstream = self.send_upstream_msg(upstream)?;

        if let Some(rsp) = downstream.response {
            if let Some(patch_rsp) = rsp.get_patch_info_rsp {
                log::info!("Latest version from server: {}", patch_rsp.latest_version);
                return Ok(patch_rsp.latest_version);
            }
        }

        Err("Failed to get latest version".into())
    }

    fn send_keep_alive(&self) -> Result<(), Box<dyn Error>> {
        let upstream = Upstream {
            request: None,
            push: Some(Push {
                data: None,
                connection_closed: None,
                keep_alive: Some(crate::proto::demux::KeepAlivePush {}),
                client_version: None,
                client_outdated: None,
                product_started: None,
                product_ended: None,
            }),
        };

        let data = upstream.encode_to_vec();
        self.send_raw(&data)?;
        Ok(())
    }

    pub fn authenticate(&self, ticket: &str, keep_alive: bool) -> Result<bool, Box<dyn Error>> {
        log::info!("Authenticating with demux server");

        let req = Req {
            request_id: self.next_request_id(),
            authenticate_req: Some(AuthenticateReq {
                token: Token {
                    ubi_ticket: Some(ticket.to_string()),
                    orbit_token: None,
                    ubi_token: None,
                },
                send_keep_alive: Some(keep_alive),
                client_id: Some("uplay_pc".to_string()),
                logout_push_group_id: None,
            }),
            get_patch_info_req: None,
            service_request: None,
            open_connection_req: None,
            client_ip_override: None,
        };

        let upstream = Upstream {
            request: Some(req),
            push: None,
        };

        let downstream = self.send_upstream_msg(upstream)?;

        if let Some(rsp) = downstream.response {
            if let Some(auth_rsp) = rsp.authenticate_rsp {
                log::info!("Authentication result: {}", auth_rsp.success);
                return Ok(auth_rsp.success);
            }
        }

        Err("Unexpected response to authenticate request".into())
    }

    pub fn open_connection(&self, service_name: &str) -> Result<u32, Box<dyn Error>> {
        log::info!("Opening connection to service: {}", service_name);

        let req = Req {
            request_id: self.next_request_id(),
            authenticate_req: None,
            get_patch_info_req: None,
            service_request: None,
            open_connection_req: Some(OpenConnectionReq {
                service_name: service_name.to_string(),
            }),
            client_ip_override: None,
        };

        let upstream = Upstream {
            request: Some(req),
            push: None,
        };

        let downstream = self.send_upstream_msg(upstream)?;

        if let Some(rsp) = downstream.response {
            if let Some(conn_rsp) = rsp.open_connection_rsp {
                if conn_rsp.success {
                    log::info!("Connection opened with ID: {}", conn_rsp.connection_id);
                    return Ok(conn_rsp.connection_id);
                } else {
                    return Err(format!("Failed to open connection to {}", service_name).into());
                }
            }
        }

        Err("Unexpected response to open connection request".into())
    }

    pub fn send_service_data(
        &self,
        connection_id: u32,
        data: &[u8],
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut prefixed_data = Vec::with_capacity(4 + data.len());
        prefixed_data.extend_from_slice(&(data.len() as u32).to_be_bytes());
        prefixed_data.extend_from_slice(data);

        let upstream = Upstream {
            request: None,
            push: Some(Push {
                data: Some(DataMessage {
                    connection_id,
                    data: prefixed_data,
                }),
                connection_closed: None,
                keep_alive: None,
                client_version: None,
                client_outdated: None,
                product_started: None,
                product_ended: None,
            }),
        };

        let send_data = upstream.encode_to_vec();
        self.send_raw(&send_data)?;

        loop {
            let response_data = self.recv_raw()?;
            let downstream = Downstream::decode(response_data.as_slice())?;

            if let Some(ref push) = downstream.push {
                if push.keep_alive.is_some() {
                    log::debug!("Received keep-alive, responding...");
                    self.send_keep_alive()?;
                    continue;
                }

                if let Some(ref data_msg) = push.data {
                    if data_msg.connection_id == connection_id {
                        let raw_data = &data_msg.data;
                        if raw_data.len() < 4 {
                            return Err("Service response too short".into());
                        }
                        let len = u32::from_be_bytes([
                            raw_data[0],
                            raw_data[1],
                            raw_data[2],
                            raw_data[3],
                        ]) as usize;
                        if raw_data.len() < 4 + len {
                            return Err(format!(
                                "Service response truncated: expected {} bytes, got {}",
                                len,
                                raw_data.len() - 4
                            )
                            .into());
                        }
                        return Ok(raw_data[4..4 + len].to_vec());
                    }
                }

                if push.connection_closed.is_some() {
                    return Err("Connection was closed by server".into());
                }

                if push.client_outdated.is_some() {
                    return Err("Client version is outdated".into());
                }
            }

            log::debug!("Received non-data downstream: {:?}", downstream);
        }
    }
}
