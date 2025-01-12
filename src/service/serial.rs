use std::{
    error::Error,
    sync::{Arc, Mutex, OnceLock},
};

use chrono::Timelike;

use crate::{
    config::{update_config_and_client, ComponentKind, CONFIG},
    constants::{SERIAL_MESSAGE_END, SERIAL_MESSAGE_INNER_SEP, SERIAL_MESSAGE_SEP},
    log_error, log_info, log_print, log_warn,
    service::interaction::{do_button, do_potentiometer},
    tcp,
};

pub static SERIAL: OnceLock<Mutex<Serial>> = OnceLock::new();

pub struct Serial {
    port: Option<Arc<Mutex<Box<dyn serialport::SerialPort>>>>,
}

struct Message(String);

impl Default for Serial {
    fn default() -> Self {
        Self { port: None }
    }
}

impl Serial {
    fn detect_device_and_connect(&mut self) -> bool {
        let mut port_not_found = false;

        let hid_api = hidapi::HidApi::new().expect("Failed to create HID API instance!");

        let available_hids = hid_api.device_list();
        let available_ports =
            serialport::available_ports().expect("Failed to retrieve serial ports!");

        let mut config = CONFIG
            .get()
            .expect("Could not retrieve CONFIG data!")
            .lock()
            .unwrap();

        // Even though we can store the port_name on the first time the device was found and
        // connected, in linux(and perhaps all unix-like OSs), that wouldn't work! since
        // everytime an app is using a port, if the device was removed, that port no longer exists
        // and would be available the next time, said device was disconnected, then reconnected.
        // We could also use /dev/serial/by-id/ but... nah! maybe later :D
        if !config.settings.port_name.is_empty() {
            // If port_name isn't empty, ignore checking by the device_name
            match self.try_connect_to_port(&config.settings.port_name, config.settings.baud_rate) {
                Ok(_) => return true,
                Err(e) => {
                    port_not_found = true;

                    log_error!(
                        "Could not connect to port `{}`: {}",
                        config.settings.port_name,
                        e
                    );
                }
            }
        }

        if port_not_found {
            log_info!(
                "Trying to find the device by name `{}`...",
                config.settings.device_name
            );
        }

        // Finding device by device_name
        for hid in available_hids {
            if hid.product_string() != Some(&config.settings.device_name) {
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

                        let port_name = &port.port_name;

                        // Update config and tell the client to reload
                        update_config_and_client(&mut config, |c| {
                            c.settings.port_name = port_name.clone()
                        });

                        match self.try_connect_to_port(port_name, config.settings.baud_rate) {
                            Ok(_) => return true,
                            Err(e) => {
                                log_error!("{}", e);

                                return false;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        log_error!(
            "Could not find a device named `{}`",
            config.settings.device_name
        );

        false
    }

    fn try_connect_to_port(
        &mut self,
        port_name: &str,
        baud_rate: u32,
    ) -> Result<(), Box<dyn Error>> {
        let timeout = std::time::Duration::from_millis(10);

        let serial_port = serialport::new(port_name, baud_rate)
            .timeout(timeout)
            .open();

        match serial_port {
            Ok(p) => {
                let mut serial_port = p;

                // This should be true for windows to start reading the serial messages
                serial_port.write_data_terminal_ready(true).unwrap();

                self.port = Some(Arc::new(Mutex::new(serial_port)));

                log_info!(
                    "A successful connection was established with `{}` at a baud rate of `{}`",
                    port_name,
                    baud_rate
                );

                Ok(())
            }
            Err(e) => Err(Box::new(e)),
        }
    }

    pub fn write(&mut self, message: String) {
        if self.port.is_none() {
            log_error!("Serial port isn't connected!");

            return;
        }

        // Messages should end with new_line '\n'
        let format_message = message.clone() + "\n";

        match self
            .port
            .as_mut()
            .unwrap()
            .lock()
            .unwrap()
            .write(format_message.as_bytes())
        {
            Ok(_) => log_info!("[OUTGOING] Message `{}` was sent over `serial`.", message),
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
            Err(e) => log_error!("{:?}", e),
        }
    }

    pub fn handle_serial_port(&mut self) {
        // Function to set `is_device_paired` in `SERVER_DATA`
        let update_pairing_status = |state: bool| {
            if let Ok(mut data) = tcp::get_server_data().lock() {
                let mut server_data = data.clone();

                server_data.is_device_paired = state;

                *data = server_data;
            }
        };
        // Device and software pairing status
        let mut paired = false;
        update_pairing_status(paired);

        while !self.detect_device_and_connect() {
            log_warn!("Could not connect to any serial devices, retrying...");

            std::thread::sleep(std::time::Duration::from_millis(1000));
        }

        let port = self.port.clone().unwrap();
        let mut buf: Vec<u8> = vec![0; 32];

        let mut message = Message::new();

        // Clear the input buffer to avoid bugs such as initializing the firmware twice.
        // If the app was closed before reading the message inside the buffer,
        // the message would remain in the buffer, potentially causing dual initialization.
        port.lock()
            .unwrap()
            .clear(serialport::ClearBuffer::Input)
            .expect("Failed to discard input buffer");

        loop {
            match port.lock().unwrap().read(buf.as_mut_slice()) {
                Ok(t) => message.push(std::str::from_utf8(&buf[..t]).unwrap()),
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
                Err(e) => {
                    log_error!("Connection was lost: {:?}", e);

                    break self.handle_serial_port();
                }
            }

            // Handle message which `TCP client` requested to be sent via serial
            if let Ok(mut data) = tcp::get_server_data().lock() {
                let mut server_data = data.clone();

                if !server_data.pending_serial_message.is_empty() {
                    self.write(server_data.pending_serial_message);

                    server_data.pending_serial_message = String::new();

                    *data = server_data;
                }
            }

            let (ready, key, value) = message.parse();

            if ready {
                let mut valid = false;
                let mut component = ComponentKind::None;
                let mut id = 0;
                let mut modkey = false;

                match key.as_str() {
                    "READY" => {
                        log_print!("[INCOMING] key: {} | value: {}", key, value);

                        self.write("c1".to_string()); // `c` => Connection, `1` => true
                    }
                    "PAIRED" => {
                        log_print!("[INCOMING] key: {} | value: {}", key, value);

                        paired = true;
                        update_pairing_status(paired);

                        // Ask device to send its internal components and memory-saved-layout
                        self.write("idata".to_string()); // `i` => Internal, `data` => device's data
                    }
                    "ERROR" => {
                        // Handle device's errors
                        log_error!("[INCOMING] ERROR: {}", value);
                    }
                    "DATA" => {
                        // Data that `software` request is handled here
                        let data_key = value.chars().nth(0).unwrap_or('\0');
                        let data_value = &value[1..];

                        match data_key {
                            // 'b' => Buttons
                            'b' => {
                                if let Ok(mut data) = tcp::get_server_data().lock() {
                                    let mut server_data = data.clone();

                                    server_data.raw_layout.0 = data_value.to_string();

                                    *data = server_data;
                                }
                            }
                            // 'p' => Potentiometers
                            'p' => {
                                if let Ok(mut data) = tcp::get_server_data().lock() {
                                    let mut server_data = data.clone();

                                    server_data.raw_layout.1 = data_value.to_string();

                                    *data = server_data;
                                }
                            }
                            _ => {}
                        }
                    }
                    "REQUEST" => {
                        let (request_key, request_value) = value
                            .split_once(SERIAL_MESSAGE_INNER_SEP)
                            .unwrap_or((&value, ""));

                        log_info!(
                            "[REQUESTED] key: {} | value: {}",
                            request_key,
                            request_value
                        );

                        let mut config = CONFIG
                            .get()
                            .expect("Could not retrieve CONFIG data!")
                            .lock()
                            .unwrap();

                        match request_key {
                            // Device is requesting startup data such as `time` and `profiles`
                            "STARTUP" => {
                                let total_seconds =
                                    chrono::Local::now().num_seconds_from_midnight();
                                let date = chrono::Local::now().format("%b. %d").to_string();

                                let profiles = {
                                    let mut profiles_string = String::new();

                                    for profile in &config.profiles {
                                        profiles_string.push_str(profile.name.as_str());
                                        profiles_string.push_str(SERIAL_MESSAGE_INNER_SEP);
                                    }

                                    profiles_string
                                };

                                self.write(format!("t{}", total_seconds)); // Time
                                self.write(format!("d{}", date)); // Date
                                self.write(format!("p{}", profiles)); // Profiles
                                self.write(format!(
                                    "P{}",
                                    config.settings.current_profile.to_string()
                                )); // Current profile
                            }
                            "profile" => {
                                let selected_profile = request_value.parse::<usize>().unwrap_or(0);

                                update_config_and_client(&mut config, |c| {
                                    c.settings.current_profile = selected_profile;
                                });
                            }
                            _ => (),
                        }
                    }
                    _ => {
                        // Format: e.g. key: bm5 -> b=button m/M=modkey 5=id
                        component = match key.chars().nth(0).unwrap_or('\0') {
                            'b' => ComponentKind::Button,
                            'p' => ComponentKind::Potentiometer,
                            _ => ComponentKind::None,
                        };
                        id = key[2..].trim().parse::<u8>().unwrap_or(0);

                        match key.chars().nth(1).unwrap_or('\0') {
                            'm' => modkey = false,
                            'M' => modkey = true,
                            _ => {}
                        }

                        // if for some reason id was zero, ignore them
                        if id != 0 {
                            valid = true;
                        }
                    }
                }

                if !paired || !valid {
                    continue;
                }

                log_print!(
                    "[INCOMING] `{}`: `{}` | modkey: `{}` | value: `{}`",
                    component,
                    id,
                    modkey,
                    value
                );

                match component {
                    ComponentKind::Button => {
                        do_button(id, value.parse::<i8>().unwrap_or(0), modkey, self)
                    }
                    ComponentKind::Potentiometer => {
                        do_potentiometer(id, value.parse::<u8>().unwrap_or(0))
                    }
                    _ => (),
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

impl Message {
    fn new() -> Self {
        Message(String::new())
    }

    fn push(&mut self, message: &str) {
        self.0.push_str(message);
    }

    fn parse(
        &mut self,
    ) -> (
        bool,   /* ready */
        String, /* key */
        String, /* value */
    ) {
        // Sometimes the incoming serial message could split into multiple messages,
        // so we check if the message is ready before using it; Parser will make sure
        // all parts of the message is combined and separated into keys and values.
        let mut ready = false;
        let mut key = String::new();
        let mut value = String::new();

        if !self.0.is_empty() && self.0.contains(SERIAL_MESSAGE_END) && !ready {
            ready = true;

            // Separate the key from the value by SERIAL_MESSAGE_SEP
            if let Some((k, raw_v)) = self.0.split_once(SERIAL_MESSAGE_SEP) {
                key = k.to_string();

                // Remove the SERIAL_MESSAGE_END and anything after that
                if let Some((v, next_message)) = raw_v.split_once(SERIAL_MESSAGE_END) {
                    value = v.to_string();

                    self.0 = next_message.to_string();
                }
            }
        }

        (ready, key, value)
    }
}

pub fn init() {
    let serial = Serial::default();

    SERIAL.get_or_init(|| Mutex::new(serial));
}
