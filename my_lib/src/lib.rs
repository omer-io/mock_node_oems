use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use std::thread::Builder;

use fastwebsockets::{upgrade, OpCode, WebSocketError, FragmentCollector};
use tokio::net::TcpListener;
use crossbeam_channel::{bounded, unbounded, Sender, Receiver};

use http_body_util::Empty;
use hyper::body::Bytes;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::Request;
use hyper::Response;
use structs::{SharedState, State};
use tokio::sync::RwLock;

pub mod structs;

type ClientId = SocketAddr;
type TxToApp = Sender<(ClientId, Vec<u8>)>;
type RxFromServer = Receiver<(ClientId, Vec<u8>)>;

pub async fn start_server() -> Result<(SharedState, RxFromServer), WebSocketError> {

    let (send_to_app, recv_from_server) = unbounded::<(ClientId, Vec<u8>)>();

    let state = Arc::new(RwLock::new(State {
        clients: HashMap::new(),
    }));

    let state_clone = state.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap();

        rt.block_on(async move {
            let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
            println!("Server started, listening on {}", "127.0.0.1:3000");
            loop {
                let (stream, addr) = listener.accept().await.unwrap();
                println!("Client connected: {}", addr);
                let state = state_clone.clone();
                let send_to_app = send_to_app.clone(); 
                tokio::spawn(async move {
                    let io = hyper_util::rt::TokioIo::new(stream);
                    let service = service_fn(move |req| {
                        server_upgrade(req, addr, state.clone(), send_to_app.clone())
                    });
                    let conn_fut = http1::Builder::new()
                        .serve_connection(io, service)
                        .with_upgrades();
                    if let Err(e) = conn_fut.await {
                    println!("An error occurred: {:?}", e);
                    }
                });
            }
        });
    });

    Ok((state, recv_from_server))

}

async fn server_upgrade(
    mut req: Request<Incoming>,
    addr: SocketAddr,
    state: SharedState,
    send_to_app: TxToApp,
) -> Result<Response<Empty<Bytes>>, WebSocketError> {
    let (response, fut) = upgrade::upgrade(&mut req)?;

    tokio::task::spawn(async move {
    let mut ws = FragmentCollector::new(fut.await.unwrap());
    handle_client(ws, addr, &state, send_to_app).await.unwrap();

    {
        let mut state = state.write().await;
        state.clients.remove(&addr);
    }

    });

    Ok(response)
}

async fn handle_client(mut ws: FragmentCollector<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>, addr: SocketAddr, state: &SharedState, to_app: TxToApp) -> Result<(), WebSocketError> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    {
        let mut state = state.write().await;
        state.clients.insert(addr, tx);
    }

    loop {
        tokio::select! {
            frame = ws.read_frame() => {
                let frame = frame?;

                match frame.opcode {
                    OpCode::Close => {
                        break;
                    }
                    OpCode::Text => {
                        let text = String::from_utf8(frame.payload.to_vec()).unwrap();
                        println!("Received text from {}: {}", addr, text);
                        // Send the message to the application layer
                        if let Err(e) = to_app.send((addr, text.into_bytes())) {
                            eprintln!("Failed to send message to app: {}", e);
                        }
                    }
                    OpCode::Binary => {
                        println!("Received binary data from {}: {:?}", addr, frame.payload);
                        // Send the binary data to the application layer
                        if let Err(e) = to_app.send((addr, frame.payload.to_vec())) {
                            eprintln!("Failed to send binary data to app: {}", e);
                        }
                    }
                    _ => {}
                }
            },
            frame = rx.recv() => {
                if let Some(frame) = frame {
                    println!("lib: Sending message to client {}: {:?}", addr, frame);
                    ws.write_frame(frame.to_frame()).await?;
                } else {
                    println!("lib: Channel closed, breaking loop for client {}", addr);
                    break;
                }
            }
        }
    }
  Ok(())
}
