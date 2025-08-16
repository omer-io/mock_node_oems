use my_lib::WebSocketServer;
use std::thread;
use std::time::Duration;
use env_logger;

fn main() {
    env_logger::init(); 
    let websocket_config = my_lib::Config {
        websocket_ip: "127.0.0.1".to_string(),
        websocket_port: 3000,
        ping_interval_secs: 5,
        requests_per_second: 1000,
        max_connections_per_ip: 1000,
        max_missed_pings_before_disconnect: 3,
    };

    let mut ws = WebSocketServer::start_server(websocket_config);

    loop {
        match ws.recv_from_server.recv() {
            Ok((client_id, payload)) => {
                println!("App: Received message from client {}: {:?}", client_id, payload);
                // let msg = my_lib::WsMessage::Binary("hello from app".to_string().into_bytes());
                let msg_string = my_lib::WsMessage::Text("hello from app".to_string());
                // if let Some(tx) = ws.clients.get(&client_id) {
                //     if let Err(e) = tx.send(msg.clone()) {
                //         eprintln!("Client {} outbound channel full or closed: {:?}", client_id, e);
                //     }
                // } else {
                //     eprintln!("Client {} not found for sending", client_id);
                // }
                let _ = ws.send_to_server.send((client_id, msg_string));
            }
            Err(_) => break,
        }
    }
}