use my_lib::WebSocketServer;
use std::thread;
use std::time::Duration;
use env_logger;

fn main() {
    env_logger::init(); 
    let websocket_config = my_lib::structs::Config {
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
            Ok((client_id, msg)) => {
                println!("App: Received message from client {}: {:?}", client_id, msg);
                let msg = my_lib::structs::WsMessage::Binary("hello from app".to_string().into_bytes());
                let msg_string = my_lib::structs::WsMessage::Text("hello from app".to_string());
                println!("App: Sending message to client {}: {:?}", client_id, msg_string);
                ws.send_to_client(client_id, msg);
            }
            Err(_) => {
                println!("breaking loop, channel closed");
                break
            } // Channel closed
        }
    }
}