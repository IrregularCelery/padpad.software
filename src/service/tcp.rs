use std::{
    io::{ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
};

use crate::config::{TCP_BUFFER_SIZE, TCP_SERVER_ADDR, TCP_SERVER_PORT};

pub fn handle_tcp_server() {
    let address = format!("{}:{}", TCP_SERVER_ADDR, TCP_SERVER_PORT);

    let listener = TcpListener::bind(&address).expect("TCP server could not bind to address!");

    println!("TCP server is running on {}", &address);

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("TCP new connection established.");
                std::thread::spawn(move || {
                    // Handle clients
                    let mut buffer = vec![0; TCP_BUFFER_SIZE];

                    loop {
                        match stream.read(&mut buffer) {
                            Ok(0) => {
                                client_disconnected();

                                break;
                            }
                            // Windows handles this differently
                            Err(e)
                                if e.kind() == ErrorKind::ConnectionReset
                                    || e.kind() == ErrorKind::BrokenPipe =>
                            {
                                client_disconnected();

                                break;
                            }
                            Ok(bytes_read) => {
                                let message = String::from_utf8_lossy(&buffer[..bytes_read]);

                                let message_trimmed = message.trim();

                                println!("Received message: {}", message_trimmed);

                                // Send a response back to the client
                                let response = message_trimmed;

                                stream.write_all(response.as_bytes()).unwrap();
                            }
                            Err(e) => {
                                eprintln!("TCP had an error while reading from stream: {}", e);

                                break;
                            }
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!("TCP connection failed: {}", e);
            }
        }
    }
}

fn client_disconnected() {
    println!("Client disconnected.");
}
