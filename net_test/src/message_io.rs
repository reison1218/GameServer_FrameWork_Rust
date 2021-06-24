use bincode::ErrorKind;
use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{self};

use std::collections::HashMap;
use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum FromClientMessage {
    Ping,
}

#[derive(Serialize, Deserialize)]
pub enum FromServerMessage {
    Pong(usize), // Used for connection oriented protocols
    UnknownPong, // Used for non-connection oriented protocols
}

struct ClientInfo {
    count: usize,
}

pub fn run(transport: Transport, addr: SocketAddr) {
    let (handler, listener) = node::split::<()>();

    let mut clients: HashMap<Endpoint, ClientInfo> = HashMap::new();

    match handler.network().listen(transport, addr) {
        Ok((_id, real_addr)) => println!("Server running at {} by {}", real_addr, transport),
        Err(_) => return println!("Can not listening at {} by {}", addr, transport),
    }
    let handler = handler.clone();

    listener.for_each(move |event| match event.network() {
        NetEvent::Connected(endpoint, success) => (), // Only generated at connect() calls.
        NetEvent::Accepted(endpoint, _listener_id) => {
            // Only connection oriented protocols will generate this event
            clients.insert(endpoint, ClientInfo { count: 0 });
            println!(
                "Client ({}) connected (total clients: {})",
                endpoint.addr(),
                clients.len()
            );
        }
        NetEvent::Message(endpoint, input_data) => {
            let message: Result<FromClientMessage, Box<ErrorKind>> =
                bincode::deserialize(&input_data);
            if let Ok(message) = message {
                match message {
                    FromClientMessage::Ping => {
                        let message = match clients.get_mut(&endpoint) {
                            Some(client) => {
                                // For connection oriented protocols
                                client.count += 1;
                                println!("Ping from {}, {} times", endpoint.addr(), client.count);
                                FromServerMessage::Pong(client.count)
                            }
                            None => {
                                // For non-connection oriented protocols
                                println!("Ping from {}", endpoint.addr());
                                FromServerMessage::UnknownPong
                            }
                        };
                        println!("server Received: {}", String::from_utf8_lossy(input_data));
                        handler.network().send(endpoint, &input_data);
                    }
                    _ => {}
                }
            }
            println!("server Received: {}", String::from_utf8_lossy(input_data));
            let output_data = bincode::serialize(&input_data).unwrap();
            handler.network().send(endpoint, &output_data);
            handler.network().remove(endpoint.resource_id());
        }
        NetEvent::Disconnected(endpoint) => {
            // Only connection oriented protocols will generate this event
            clients.remove(&endpoint).unwrap();
            println!(
                "Client ({}) disconnected (total clients: {})",
                endpoint.addr(),
                clients.len()
            );
        }
    });
}
