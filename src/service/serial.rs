use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use crate::config::{SERIAL_MESSAGE_END, SERIAL_MESSAGE_SEP};

use super::config_manager::CONFIG;

struct Serial {
    port: Option<Arc<Mutex<Box<dyn serialport::SerialPort>>>>,
}

struct Message(String);

impl Serial {
    fn detect_device_and_connect(&mut self) -> bool {
        let hid_api = hidapi::HidApi::new().expect("Failed to create HID API instance!");

        let available_hids = hid_api.device_list();
        let available_ports =
            serialport::available_ports().expect("Failed to retrieve serial ports!");

        let mut config = CONFIG
            .get()
            .expect("Could not retrieve config data!")
            .lock()
            .unwrap();

        if !config.settings.port_name.is_empty() {
            // If port_name isn't empty, ignore checking by the device_name

            match self.try_connect_to_port(&config.settings.port_name, config.settings.baud_rate) {
                Ok(_) => return true,
                Err(e) => {
                    eprintln!("{}", e);

                    return false;
                }
            }
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

                        config.update(|c| c.settings.port_name = port.port_name.clone(), true);

                        return true;
                    }
                    _ => {}
                }
            }
        }

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

                Ok(())
            }
            Err(e) => Err(Box::new(e)),
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
