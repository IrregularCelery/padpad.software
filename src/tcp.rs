use std::{
    io::{ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
};

use serde::{Deserialize, Serialize};

use crate::{
    constants::{TCP_BUFFER_SIZE, TCP_READ_TIMEOUT, TCP_SERVER_ADDR},
    log_error, log_info, log_print,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerData {
    pub is_connected: bool,
    pub order: String, // Server order message for client to do something. e.g. Reload config
}

impl ServerData {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap_or("{}".to_string())
    }

    pub fn parse(server_data_string: String) -> Self {
        serde_json::from_str(&server_data_string).unwrap_or(Self::default())
    }

    pub fn set_connected(&mut self, is_connected: bool) {
        self.is_connected = is_connected;
    }
}

impl Default for ServerData {
    fn default() -> Self {
        Self {
            is_connected: false,
            order: String::new(),
        }
    }
}

pub fn is_another_instance_running() -> bool {
    let mut another_instance_running = false;

    match TcpListener::bind(TCP_SERVER_ADDR) {
        Err(e) if e.kind() == ErrorKind::AddrInUse => another_instance_running = true,
        _ => (),
    }

    another_instance_running
}

pub fn handle_tcp_server() {
    let listener =
        TcpListener::bind(TCP_SERVER_ADDR).expect("TCP server could not bind to address!");

    log_info!("TCP server is running on {}", TCP_SERVER_ADDR);

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                log_print!("TCP new connection established.");

                std::thread::spawn(move || {
                    // Handle clients
                    let mut buffer = vec![0; TCP_BUFFER_SIZE];

                    loop {
                        match stream.read(&mut buffer) {
                            Ok(0) => {
                                log_print!("Client disconnected.");

                                break;
                            }
                            // Windows handles this differently
                            Err(e)
                                if e.kind() == ErrorKind::ConnectionReset
                                    || e.kind() == ErrorKind::BrokenPipe =>
                            {
                                log_print!("Client disconnected.");

                                break;
                            }
                            Ok(bytes_read) => {
                                let message = String::from_utf8_lossy(&buffer[..bytes_read]);

                                server_to_client_message(&mut stream, message.trim());
                            }
                            Err(e) => {
                                log_error!(
                                    "TCP server had an error while reading from stream: {}",
                                    e
                                );

                                break;
                            }
                        }

                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                });
            }
            Err(e) => {
                log_error!("TCP connection failed: {}", e);
            }
        }
    }
}

fn handle_tcp_client() -> Result<TcpStream, String> {
    let stream = match std::net::TcpStream::connect(TCP_SERVER_ADDR) {
        Ok(s) => s,
        Err(_) => {
            return Err(
                "Failed to connect to server!\nMake sure the `Service` app is running!".into(),
            )
        }
    };

    stream
        .set_read_timeout(Some(std::time::Duration::from_millis(TCP_READ_TIMEOUT)))
        .expect("Failed to set `read_timeout`");

    Ok(stream)
}

fn server_to_client_message(client_stream: &mut TcpStream, message: &str) {
    log_print!("Received message: {}", message);

    let mut response: Option<String> = None;

    match message {
        "client::get_data" => {
            // TEST: ServerData
            let test_server_data = ServerData {
                is_connected: false,
                order: "server::reload_config".to_string(),
            };

            response = Some(test_server_data.to_string());
        }
        _ => (),
    }

    if let Some(r) = response {
        log_print!("Sending a message to client: {}", r);

        client_stream.write_all(r.as_bytes()).unwrap();
    }
}

pub fn client_to_server_message(message: &str) -> Result<String, String> {
    let mut stream = match handle_tcp_client() {
        Ok(s) => s,
        Err(e) => return Err(e),
    };

    if let Err(e) = stream.write_all(message.as_bytes()) {
        println!("Failed to write to server: {:?}", e);

        return Err("Failed to send a message!\nMake sure the `Service` app is running!".into());
    }

    let mut buffer = vec![0; TCP_BUFFER_SIZE];

    match stream.read(&mut buffer) {
        Ok(0) => {
            log_print!("Server disconnected.");
        }
        // Windows handles this differently
        Err(e) if e.kind() == ErrorKind::ConnectionReset || e.kind() == ErrorKind::BrokenPipe => {
            log_print!("Server disconnected.");
        }
        Ok(bytes_read) => {
            let message = String::from_utf8_lossy(&buffer[..bytes_read]);

            return Ok(message.trim().to_string());
        }
        Err(e) => {
            let error = format!("TCP client had an error while reading from stream: {}", e);
            log_error!("{}", error);

            return Err(error);
        }
    }

    Err("There was an `Unknown` problem while sending a message!\nMake sure the `Service` app is running!".into())
}
