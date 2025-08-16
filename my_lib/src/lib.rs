use {
    std::{collections::HashMap, sync::Arc, net::SocketAddr},
    fastwebsockets::{upgrade, OpCode, WebSocketError, FragmentCollector, Frame},
    tokio::{net::TcpListener, sync::{RwLock, mpsc}},
    crossbeam_channel::{unbounded, Sender, Receiver},
    http_body_util::Empty,
    hyper::{body::Bytes, body::Incoming, server::conn::http1, service::service_fn, Request, Response},
    // structs::{SharedState, State},
    uuid::Uuid,
    tokio::time::{timeout, Duration},
    dashmap::DashMap,
};

pub type ClientId = Uuid;
// pub type SharedClients = Arc<DashMap<ClientId, mpsc::Sender<structs::WsMessage>>>;
pub type SharedClients = Arc<DashMap<ClientId, mpsc::UnboundedSender<WsMessage>>>;

type TxToApp = Sender<(ClientId, Vec<u8>)>;
type RxFromServer = Receiver<(ClientId, Vec<u8>)>;
type TxToServer = mpsc::UnboundedSender<(ClientId, WsMessage)>;

pub struct WebSocketServer {
    pub clients: SharedClients,
    pub recv_from_server: RxFromServer,
    pub send_to_server: TxToServer,
    // connections_per_ip: Arc<DashMap<String, u64>>, // Track connections per IP
}

impl WebSocketServer {
    pub fn start_server(config: Config) -> Self {

        let (send_to_app, recv_from_server) = unbounded::<(ClientId, Vec<u8>)>();
        let (send_to_server, mut recv_from_app) = mpsc::unbounded_channel::<(ClientId, WsMessage)>();

        let clients: SharedClients = Arc::new(DashMap::new());
        let clients_clone_for_runtime = clients.clone();

        // let connections_per_ip = Arc::new(DashMap::new());
        // let connections_per_ip_clone = connections_per_ip.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .unwrap();

            rt.block_on(async move {
                let listener = TcpListener::bind(format!("{}:{}", config.websocket_ip, config.websocket_port)).await.unwrap();
                println!("Server started, listening on {}:{}", config.websocket_ip, config.websocket_port);

                // Task: forward app->server messages into client senders
                let clients_for_forward = clients_clone_for_runtime.clone();
                tokio::spawn(async move {
                    while let Some((client_id, msg)) = recv_from_app.recv().await {
                        if let Some(tx) = clients_for_forward.get(&client_id) {
                            if let Err(e) = tx.send(msg.clone()) {
                                eprintln!("Client {} outbound channel full or closed: {:?}", client_id, e);
                            }
                        } else {
                            eprintln!("Client {} not found for sending", client_id);
                        }
                    }
                });

                loop {
                    let (stream, addr) = listener.accept().await.unwrap();
                    let client_id = Uuid::new_v4();
                    // let ip = addr.ip().to_string();

                    // // Enforce connection limit per IP
                    // let mut connections = connections_per_ip_clone.entry(ip.clone()).or_insert(1);
                    // if *connections >= config.max_connections_per_ip {
                    //     log::warn!("Connection limit reached for IP: {}", ip);
                    //     continue;
                    // }
                    // *connections += 1;

                    println!("Client connected: {} (UUID: {})", addr, client_id);
                    let clients_clone = clients_clone_for_runtime.clone();
                    let send_to_app = send_to_app.clone(); 

                    tokio::spawn(async move {
                        let io = hyper_util::rt::TokioIo::new(stream);
                        let service = service_fn(move |req| {
                            server_upgrade(req, client_id, clients_clone.clone(), send_to_app.clone(), config.ping_interval_secs, config.max_missed_pings_before_disconnect)
                        });
                        let conn_fut = http1::Builder::new()
                            .serve_connection(io, service)
                            .with_upgrades();
                        if let Err(e) = conn_fut.await {
                            eprintln!("An error occurred: {:?}", e);
                        }
                    });
                }
            });
        });

        Self {
            clients,
            recv_from_server,
            send_to_server,
            // connections_per_ip,
        }
    }
}

async fn server_upgrade(
    mut req: Request<Incoming>,
    client_id: ClientId,
    clients: SharedClients,
    send_to_app: TxToApp,
    ping_interval_secs: u64,
    max_missed_pings_before_disconnect: u64,
) -> Result<Response<Empty<Bytes>>, WebSocketError> {
    let (response, fut) = upgrade::upgrade(&mut req)?;

    tokio::task::spawn(async move {
    let mut ws = FragmentCollector::new(fut.await.unwrap());
    // let (tx, mut rx) = mpsc::channel::<structs::WsMessage>(1024);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    clients.insert(client_id, tx);
    handle_client(ws, client_id, &clients, send_to_app, rx, ping_interval_secs, max_missed_pings_before_disconnect).await.unwrap();

    clients.remove(&client_id);

    });

    Ok(response)
}

async fn handle_client(
    mut ws: FragmentCollector<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>, 
    client_id: ClientId, 
    clients: &SharedClients,
    to_app: TxToApp,
    mut rx: mpsc::UnboundedReceiver<WsMessage>,
    ping_interval_secs: u64,
    max_missed_pings_before_disconnect: u64,
) -> Result<(), WebSocketError> {
    let heartbeat_timeout = Duration::from_secs(ping_interval_secs * max_missed_pings_before_disconnect);
    loop {
        tokio::select! {
            // Inactivity timeout on read_frame
            result = timeout(heartbeat_timeout, ws.read_frame()) => {
                match result {
                    Ok(Ok(frame)) => {
                        match frame.opcode {
                            OpCode::Close => {
                                break;
                            }
                            OpCode::Text => {
                                let text = String::from_utf8(frame.payload.to_vec()).unwrap();
                                println!("Received text from {}: {}", client_id, text);
                                // // Send the message to the application layer
                                if let Err(e) = to_app.send((client_id, text.into_bytes())) {
                                    eprintln!("Failed to send message to app: {}", e);
                                }
                            }
                            OpCode::Binary => {
                                println!("Received binary data from {}: {:?}", client_id, frame.payload);
                                // Send the binary data to the application layer
                                if let Err(e) = to_app.send((client_id, frame.payload.to_vec())) {
                                    eprintln!("Failed to send binary data to app: {}", e);
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(Err(e)) => {
                        eprintln!("WebSocket error: {:?}", e);
                        break;
                    }
                    Err(_) => {
                        eprintln!("Client {} timed out due to inactivity", client_id);
                        let _ = ws.write_frame(fastwebsockets::Frame::close(1000, b"")).await;
                        break;
                    }
                }
            },
            frame = rx.recv() => {
                if let Some(frame) = frame {
                    println!("Sending message to client {}: {:?}", client_id, frame);
                    ws.write_frame(frame.to_frame()).await?;
                } else {
                    break;
                }
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Config {
    pub websocket_ip: String,
    pub websocket_port: u16,
    pub ping_interval_secs: u64,
    pub requests_per_second: u64,
    pub max_connections_per_ip: u64,
    pub max_missed_pings_before_disconnect: u64
}


#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
    /// Send a pong message with the given data.
    Pong(Vec<u8>),
    /// Close the connection with the given code and reason.
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