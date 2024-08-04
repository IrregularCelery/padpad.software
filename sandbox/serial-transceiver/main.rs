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
        // Sometimes the incoming serial message could split into multiple messages,
        // so we check if the message is ready before using it; Parser will make sure
        // parts of the message is combined and separated into keys and values.
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

// TODO: Move this somewhere else!
fn log(args: &std::fmt::Arguments) {
    println!("{}", args);
}

macro_rules! log {
    ($fmt:expr, $($arg:tt)*) => {
        log(&format_args!($fmt, $($arg)*))
    };
}

fn main() {
    detect_device();

    let port_name = "/dev/ttyACM0";
    let baud_rate = 38_400;
    let timeout = Duration::from_millis(10);

    // Device and software pairing status
    let mut paired = false;

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

        // Temporary LED test
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
                let mut valid = false;
                let mut component = "";
                let mut id = 0;

                match key.as_str() {
                    "READY" => {
                        log!("[INCOMING] key: {} | value: {}", key, value);

                        serial_send(&port, "p1".to_string());
                    }
                    "PAIRED" => {
                        log!("[INCOMING] key: {} | value: {}", key, value);

                        paired = true;
                    }
                    _ => {
                        component = match key.chars().nth(0).unwrap_or('\0') {
                            'b' => "Button",
                            _ => "Unknown",
                        };
                        //component = key.chars().nth(0).unwrap_or('\0');
                        id = key[1..].trim().parse::<u8>().unwrap_or(0);

                        // if for some reason the component or id were zero, ignore them
                        if !component.is_empty() && id != 0 {
                            valid = true;
                        }
                    }
                }

                if !paired || !valid {
                    continue;
                }

                log!(
                    "[INCOMING] component: `{}` id: `{}` | value: {}",
                    component,
                    id,
                    value
                );

                match key.as_str() {
                    "b1" => {
                        if !led {
                            // led=1
                            serial_send(&port, "l1".to_string());

                            led = true;
                        } else {
                            // led=0
                            serial_send(&port, "l0".to_string());

                            led = false;
                        }
                    }
                    "b2" => {
                        log!("[LOG] {}", "hi!");
                        //serial_send(&port, "s0".to_string());
                    }
                    _ => (),
                }
            }

            thread::sleep(Duration::from_millis(10));
        }
    });

    fn serial_send(port: &Arc<Mutex<Box<dyn serialport::SerialPort>>>, message: String) {
        match port.lock().unwrap().write(message.as_bytes()) {
            Ok(_) => log!("[OUTGOING] Message `{}` was sent over `serial`.", message),
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
                        log!("{} | {:?}", port.port_name, info);
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
