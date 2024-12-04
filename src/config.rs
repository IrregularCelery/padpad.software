// General
pub const DEVICE_NAME: &str = "PadPad";

// Serial
const DEFAULT_BAUD_RATE: u32 = 38_400;

// TCP
pub const TCP_SERVER_ADDR: &str = "127.0.0.1";
pub const TCP_SERVER_PORT: u32 = 51690; // Random number in range of unused ports
pub const TCP_BUFFER_SIZE: usize = 4096; // 4 KB buffer
