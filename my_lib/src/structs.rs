use std::{collections::HashMap, sync::Arc};

use fastwebsockets::Frame;
use tokio::sync::{mpsc::UnboundedSender, RwLock};
use uuid::Uuid;

pub type Tx = UnboundedSender<WsMessage>;
pub type SharedState = Arc<RwLock<State>>;
type ClientId = Uuid;

#[derive(Clone, Debug)]
pub struct Config {
    pub websocket_ip: String,
    pub websocket_port: u16,
    pub ping_interval_secs: u64,
    pub requests_per_second: u64,
    pub max_connections_per_ip: u64,
    pub max_missed_pings_before_disconnect: u64
}

pub struct State {
    pub clients: HashMap<ClientId, Tx>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),

    /// Send a pong message with the given data.
    Pong(Vec<u8>),

    /// Close the connection with the given code and reason.
    ///
    /// u16 is the status code
    /// String is the reason
    Close(u16, String),
}

impl WsMessage {
    pub fn to_frame(&self) -> Frame {
        match self {
            WsMessage::Text(text) => Frame::text(text.as_bytes().into()),
            WsMessage::Binary(data) => Frame::binary(data.as_slice().into()),
            WsMessage::Pong(data) => Frame::pong(data.as_slice().into()),
            WsMessage::Close(code, reason) => Frame::close(*code, reason.as_bytes()),
        }
    }
}