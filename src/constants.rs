// General
pub const APP_NAME: &str = "PadPad";
pub const DEFAULT_DEVICE_NAME: &str = "PadPad";
pub const CONFIG_FILE_NAME: &str = "config.toml";

// Serial
pub const DEFAULT_BAUD_RATE: u32 = 38_400;
pub const SERIAL_MESSAGE_SEP: &str = ":";
pub const SERIAL_MESSAGE_INNER_SEP: &str = "|";
pub const SERIAL_MESSAGE_END: &str = ";";

// TCP
pub const TCP_SERVER_ADDR: &str = "127.0.0.1:51690"; // Random number in range of unused ports
pub const TCP_READ_TIMEOUT: u64 = 5000; // Client waiting duration for server response (in ms)
pub const TCP_BUFFER_SIZE: usize = 4096; // 4 KB buffer
pub const SERVER_DATA_UPDATE_INTERVAL: u64 = 16; // If you don't care about the `Dashboard` being
                                                 // responsive while device components are being used
                                                 // you can have higher interval (in ms) (16 ≈ 60FPS)
                                                 // if client isn't connected, interval is 1000ms

// Dashboard
pub const DASHBOARD_DISAPLY_PIXEL_SIZE: f32 = 4.0;
pub const DASHBOARD_DEVICE_INTERNAL_PROFILE: &str = "Internal";
pub const DASHBOARD_PROFILE_MAX_CHARACTERS: usize = 10;

// Validation
//pub const FORBIDDEN_CHARACTERS: [&str; 3] = [
//    SERIAL_MESSAGE_SEP,
//    SERIAL_MESSAGE_INNER_SEP,
//    SERIAL_MESSAGE_END,
//];
pub const HOME_IMAGE_WIDTH: usize = 42;
pub const HOME_IMAGE_HEIGHT: usize = 42;
pub const HOME_IMAGE_BYTES_SIZE: usize = ((HOME_IMAGE_WIDTH + (8 - 1)) / 8) * HOME_IMAGE_HEIGHT;

// Debug
pub const DEBUG_SERIAL_DISABLE: bool = false; // Disables Serial comm for testing dashboard
pub const DEBUG_TCP_CLIENT_CONNECTION: bool = false;
pub const DEBUG_TCP_SERVER_MESSAGE_CONFIRMATION: bool = false; // Logging received/sent messages
