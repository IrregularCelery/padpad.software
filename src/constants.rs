// General
pub const APP_NAME: &str = "PadPad";
pub const DEFAULT_DEVICE_NAME: &str = "PadPad";
pub const CONFIG_FILE_NAME: &str = "config.toml";

// Serial
pub const DEFAULT_BAUD_RATE: u32 = 38_400;
pub const SERIAL_MESSAGE_SEP: &str = ":";
//pub const SERIAL_MESSAGE_INNER_SEP: &str = ":";
pub const SERIAL_MESSAGE_END: &str = ";";

// TCP
pub const TCP_SERVER_ADDR: &str = "127.0.0.1";
pub const TCP_SERVER_PORT: u32 = 51690; // Random number in range of unused ports
pub const TCP_BUFFER_SIZE: usize = 4096; // 4 KB buffer

// Validation
//pub const FORBIDDEN_CHARACTERS: [&str; 3] = [
//    SERIAL_MESSAGE_SEP,
//    SERIAL_MESSAGE_INNER_SEP,
//    SERIAL_MESSAGE_END,
//];
