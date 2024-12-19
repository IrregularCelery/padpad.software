use std::{
    io::{ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
};

use crate::{
    constants::{TCP_BUFFER_SIZE, TCP_SERVER_ADDR, TCP_SERVER_PORT},
    log_error, log_info,
};

pub fn is_another_instance_running() -> bool {
    let address = format!("{}:{}", TCP_SERVER_ADDR, TCP_SERVER_PORT);

    let mut another_instance_running = false;

    match TcpListener::bind(&address) {
        Err(e) if e.kind() == ErrorKind::AddrInUse => another_instance_running = true,
        _ => (),
    }

    another_instance_running
}

pub fn handle_tcp_server() {
    let address = format!("{}:{}", TCP_SERVER_ADDR, TCP_SERVER_PORT);

    let listener = TcpListener::bind(&address).expect("TCP server could not bind to address!");

    log_info!("TCP server is running on {}", &address);

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                log_info!("TCP new connection established.");

                std::thread::spawn(move || {
                    // Handle clients
                    let mut buffer = vec![0; TCP_BUFFER_SIZE];

                    loop {
                        match stream.read(&mut buffer) {
                            Ok(0) => {
                                log_info!("Client disconnected.");

                                break;
                            }
                            // Windows handles this differently
                            Err(e)
                                if e.kind() == ErrorKind::ConnectionReset
                                    || e.kind() == ErrorKind::BrokenPipe =>
                            {
                                log_info!("Client disconnected.");

                                break;
                            }
                            Ok(bytes_read) => {
                                let message = String::from_utf8_lossy(&buffer[..bytes_read]);

                                server_to_client_messages(&mut stream, message.trim());
                            }
                            Err(e) => {
                                log_error!("TCP had an error while reading from stream: {}", e);

                                break;
                            }
                        }
                    }
                });
            }
            Err(e) => {
                log_error!("TCP connection failed: {}", e);
            }
        }
    }
}

fn handle_tcp_client() {}

fn server_to_client_messages(client_stream: &mut TcpStream, message: &str) {
    log_info!("Received message: {}", message);

    let test = client_stream.peer_addr().unwrap();

    log_info!("{:?}", test);

    // Send a response back to the client
    let response = message;

    client_stream.write_all(response.as_bytes()).unwrap();
}
