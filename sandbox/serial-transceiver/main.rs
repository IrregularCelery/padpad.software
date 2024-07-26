use std::io;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const MESSAGE_SEP: &str = ":";
const MESSAGE_END: &str = ";";

struct Message(String);

impl Message {
    fn new() -> Self {
        Message(String::new())
    }

    fn push(&mut self, message: &str) {
        self.0.push_str(message);
    }

    fn parse(&mut self) -> (bool, String, String) {
        let mut ready = false;
        let mut key = String::new();
        let mut value = String::new();

        if !self.0.is_empty() && self.0.contains(MESSAGE_END) && !ready {
            ready = true;

            // Separate the key from the value by MESSAGE_SEP
            if let Some((k, raw_v)) = self.0.split_once(MESSAGE_SEP) {
                key = k.to_string();

                // Remove the MESSAGE_END and anything after that
                if let Some((v, next_message)) = raw_v.split_once(MESSAGE_END) {
                    value = v.to_string();

                    self.0 = next_message.to_string();
                }
            }
        }

        (ready, key, value)
    }
}

fn main() {
    detect_device();

    let port_name = "/dev/ttyACM0";
    let baud_rate = 38_400;
    let timeout = Duration::from_millis(10);

    let mut firmware_init = false;

    let serial_port = serialport::new(port_name, baud_rate)
        .timeout(timeout)
        .open();

    let port = Arc::new(Mutex::new(
        serial_port.expect("Port connection was not successful!"),
    ));

    let read_port = port.clone();
    let write_port = port.clone();

    let read_thread = thread::spawn(move || {
        let mut buf: Vec<u8> = vec![0; 32];

        let mut message = Message::new();

        let mut led = false;

        // Clear the input buffer to avoid bugs such as initializing the firmware twice.
        // If the app was closed before reading the message inside the buffer,
        // the message would remain in the buffer, potentially causing dual initialization.
        port.lock()
            .unwrap()
            .clear(serialport::ClearBuffer::Input)
            .expect("Failed to discard input buffer");

        loop {
            match read_port.lock().unwrap().read(buf.as_mut_slice()) {
                Ok(t) => message.push(std::str::from_utf8(&buf[..t]).unwrap()),
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                Err(e) => eprintln!("{:?}", e),
            }

            let (ready, key, value) = message.parse();

            if ready {
                if key == "INIT" {
                    firmware_init = true;
                }

                if !firmware_init {
                    continue;
                }

                match key.as_str() {
                    "b1" => {
                        if !led {
                            serial_send(&port, "l".to_string());

                            led = true;
                        } else {
                            serial_send(&port, "L".to_string());

                            led = false;
                        }
                    }
                    "b2" => println!("Button 2 was clicked!"),
                    _ => (),
                }

                println!("key: {} | value: {}", key, value);
            }

            thread::sleep(Duration::from_millis(10));
        }
    });

    fn serial_send(write_port: &Arc<Mutex<Box<dyn serialport::SerialPort>>>, message: String) {
        match write_port.lock().unwrap().write(message.as_bytes()) {
            Ok(_) => println!("Message `{}` was sent.", message),
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            Err(e) => eprintln!("{:?}", e),
        }
    }

    let write_thread = thread::spawn(move || loop {
        use std::io::{stdin, stdout, Write};
        let mut s = String::new();
        let _ = stdout().flush();

        stdin()
            .read_line(&mut s)
            .expect("Did not enter a correct string");

        s.pop()
            .expect("Couldn't remove the `NEWLINE` character!")
            .to_string();

        serial_send(&write_port, s);
    });

    read_thread
        .join()
        .expect_err("there was a problem while spawning the `read` thread!");
    write_thread
        .join()
        .expect_err("there was a problem while spawning the `write` thread!");
}

fn detect_device() -> Option<String> {
    match get_available_ports() {
        Some(ports) => {
            for port in ports {
                match port.port_type {
                    serialport::SerialPortType::UsbPort(info) => {
                        println!("{} | {:?}", port.port_name, info);
                    }
                    _ => {}
                }
            }

            return Some(String::from("TODO"));
        }
        None => return Some(String::from("No ports found!")),
    }
}

fn get_available_ports() -> Option<Vec<serialport::SerialPortInfo>> {
    match serialport::available_ports() {
        Ok(ports) => return Some(ports),
        Err(_) => return None,
    }
}
