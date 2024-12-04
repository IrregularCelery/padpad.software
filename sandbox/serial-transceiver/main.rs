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
    let serial_thread = thread::spawn(|| {
        handle_serial_port();
    });

    serial_thread
        .join()
        .expect_err("there was a problem while spawning the `read` thread!");
}

const DEVICE_NAME: &str = "PadPad";
const DEFAULT_BAUD_RATE: u32 = 38_400;

fn detect_device() -> (String, serialport::UsbPortInfo) {
    let hid_api = hidapi::HidApi::new().expect("Failed to create HID API instance!");

    let available_hids = hid_api.device_list();
    let available_ports = serialport::available_ports().expect("Failed to retrieve serial ports!");

    let mut device = (
        "".to_string(),
        serialport::UsbPortInfo {
            vid: 0,
            pid: 0,
            serial_number: Some("".to_string()),
            manufacturer: Some("".to_string()),
            product: Some("".to_string()),
        },
    );

    for hid in available_hids {
        if hid.product_string() != Some(DEVICE_NAME) {
            continue;
        }

        for port in &available_ports {
            match &port.port_type {
                serialport::SerialPortType::UsbPort(port_info) => {
                    let port_serial_number = port_info.serial_number.clone();
                    let hid_serial_number = hid.serial_number().unwrap().to_string();

                    if port_serial_number != Some(hid_serial_number) {
                        continue;
                    }

                    device.0 = port.port_name.clone();
                    device.1.vid = hid.vendor_id();
                    device.1.pid = hid.product_id();
                    device.1.serial_number = Some(hid.serial_number().unwrap().to_string());
                    device.1.manufacturer = Some(hid.manufacturer_string().unwrap().to_string());
                    device.1.product = Some(hid.product_string().unwrap().to_string());

                    return device;
                }
                _ => {}
            }
        }
    }

    device
}

fn try_connect_to_port(
    port_name: &str,
    baud_rate: u32,
) -> Option<Arc<Mutex<Box<dyn serialport::SerialPort>>>> {
    let timeout = Duration::from_millis(10);

    let serial_port = serialport::new(port_name, baud_rate)
        .timeout(timeout)
        .open();

    let mut port: Option<Arc<Mutex<Box<dyn serialport::SerialPort>>>> = None;

    match serial_port {
        Ok(p) => {
            let mut serial_port = p;

            // This should be true for windows to start reading the serial messages
            serial_port.write_data_terminal_ready(true).unwrap();

            port = Some(Arc::new(Mutex::new(serial_port)));
        }
        _ => (),
    }

    port
}

fn serial_send(port: &Arc<Mutex<Box<dyn serialport::SerialPort>>>, message: String) {
    match port.lock().unwrap().write(message.as_bytes()) {
        Ok(_) => log!("[OUTGOING] Message `{}` was sent over `serial`.", message),
        Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
        Err(e) => eprintln!("{:?}", e),
    }
}

fn handle_serial_port() {
    let baud_rate = DEFAULT_BAUD_RATE;

    // Device and software pairing status
    let mut paired = false;

    let mut tried_port: Option<Arc<Mutex<Box<dyn serialport::SerialPort>>>> = None;

    while tried_port.is_none() {
        let (port_name, _port_info) = detect_device();

        match try_connect_to_port(&port_name, baud_rate) {
            Some(p) => {
                println!(
                    "A successful connection was established with `{}` at a baud rate of `{}`",
                    port_name, baud_rate
                );

                tried_port = Some(p);
            }
            None => {
                println!(
                    "Could not find any device named `{}`, Retrying...",
                    DEVICE_NAME
                );

                std::thread::sleep(std::time::Duration::from_millis(1000));
            }
        }
    }

    let port = tried_port.unwrap();

    let read_port = port.clone();
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
            Err(e) => {
                eprintln!("Connection was lost: {:?}", e);

                break handle_serial_port();
            }
        }

        //println!("test");

        let (ready, key, value) = message.parse();

        if ready {
            //log!("{}, {}", key, value);

            let mut valid = false;
            let mut component = "";
            let mut id = 0;
            let mut modkey = false;

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
                    id = key[2..].trim().parse::<u8>().unwrap_or(0);

                    match key.chars().nth(1).unwrap_or('\0') {
                        'm' => modkey = false,
                        'M' => modkey = true,
                        _ => {}
                    }

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
                "[INCOMING] `{}` `{}` | modkey: {} | value: {}",
                component,
                id,
                modkey,
                value
            );

            if component == "Button" {
                match id {
                    1 => {
                        if !modkey {
                            match value.as_str() {
                                "1" => {
                                    log!("{}", "what?");
                                    serial_send(&port, "l1".to_string());

                                    led = true;
                                }
                                "0" => {
                                    serial_send(&port, "l0".to_string());

                                    led = false;
                                }
                                _ => {}
                            }
                            //if !led {
                            //    // led=1
                            //    serial_send(&port, "l1".to_string());
                            //
                            //    led = true;
                            //} else {
                            //    // led=0
                            //    serial_send(&port, "l0".to_string());
                            //
                            //    led = false;
                            //}
                        }
                    }
                    _ => {}
                }
            }

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
}
