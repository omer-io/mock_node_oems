use my_lib::start_server;
use std::thread;
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Start the server and get shared state and receiver for app messages
    let (state, recv_from_server) = start_server().await.unwrap();

    loop {
        match recv_from_server.recv() {
            Ok((client_id, msg)) => {
                let state_guard = state.read().await;
                if let Some(tx) = state_guard.clients.get(&client_id) {
                    let msg = my_lib::structs::WsMessage::Text("hello from app".to_string());
                    println!("App: Sending message to client {}: {:?}", client_id, msg);
                    if let Err(e) = tx.send(msg) {
                        println!("App: Failed to send message to client {}: {:?}", client_id, e);
                    }
                }
            }
            Err(_) => {
                println!("breaking loop, channel closed");
                break
            } // Channel closed
        }
    }
}