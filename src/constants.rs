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
pub const DASHBOARD_DISAPLY_PIXEL_SIZE: f32 = 3.25;
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
pub const HOME_IMAGE_DEFAULT_BYTES: &str = "0000000000FC0000FE0100FC00E0FF1F00FC00F8FF7F00FC00FCFFFF00FC00FFFFFF03FC80FFFFFF07FCC0FF03FF0FFCE0FF00FC1FFCE07F00F81FFCF03F00F03FFCF83F00F07FFCF81F00E07FFCFC1F00E0FFFCFC1F00E0FFFCFC1F00E0FFFCFC1F00E0FFFCFE1F00E0FFFDFE3F00F0FFFDFE3F00F0FFFDFE7F00F8FFFDFEFF00FCFFFDFEFF03FFFFFDFEFFFFFFFFFDFEFFFFFFFFFDFCFFFFFFFFFCFCFF00FCFFFCFC0F00C0FFFCFC010000FEFCF80000007CFC7800000078FC7000000038FCE00000001CFCE00100001EFCC00300000FFC800F00C007FC007F00F803FC00FC03FF00FC00F8FF7F00FC00E0FF1F00FC0000FE0100FC0000000000FC";

// Debug
pub const DEBUG_SERIAL_DISABLE: bool = false; // Disables Serial comm for testing dashboard
pub const DEBUG_TCP_CLIENT_CONNECTION: bool = false;
pub const DEBUG_TCP_SERVER_MESSAGE_CONFIRMATION: bool = false; // Logging received/sent messages

pub static KEYS: [enigo::Key; 87] = [
    enigo::Key::Control,
    enigo::Key::Shift,
    enigo::Key::Alt,
    enigo::Key::Meta,
    enigo::Key::Unicode('a'),
    enigo::Key::Unicode('b'),
    enigo::Key::Unicode('c'),
    enigo::Key::Unicode('d'),
    enigo::Key::Unicode('e'),
    enigo::Key::Unicode('f'),
    enigo::Key::Unicode('g'),
    enigo::Key::Unicode('h'),
    enigo::Key::Unicode('i'),
    enigo::Key::Unicode('j'),
    enigo::Key::Unicode('k'),
    enigo::Key::Unicode('l'),
    enigo::Key::Unicode('m'),
    enigo::Key::Unicode('n'),
    enigo::Key::Unicode('o'),
    enigo::Key::Unicode('p'),
    enigo::Key::Unicode('q'),
    enigo::Key::Unicode('r'),
    enigo::Key::Unicode('s'),
    enigo::Key::Unicode('t'),
    enigo::Key::Unicode('u'),
    enigo::Key::Unicode('v'),
    enigo::Key::Unicode('w'),
    enigo::Key::Unicode('x'),
    enigo::Key::Unicode('y'),
    enigo::Key::Unicode('z'),
    enigo::Key::Unicode('0'),
    enigo::Key::Unicode('1'),
    enigo::Key::Unicode('2'),
    enigo::Key::Unicode('3'),
    enigo::Key::Unicode('4'),
    enigo::Key::Unicode('5'),
    enigo::Key::Unicode('6'),
    enigo::Key::Unicode('7'),
    enigo::Key::Unicode('8'),
    enigo::Key::Unicode('9'),
    enigo::Key::Return,
    enigo::Key::Tab,
    enigo::Key::Backspace,
    enigo::Key::Escape,
    enigo::Key::Space,
    enigo::Key::CapsLock,
    enigo::Key::LeftArrow,
    enigo::Key::RightArrow,
    enigo::Key::UpArrow,
    enigo::Key::DownArrow,
    enigo::Key::F1,
    enigo::Key::F2,
    enigo::Key::F3,
    enigo::Key::F4,
    enigo::Key::F5,
    enigo::Key::F6,
    enigo::Key::F7,
    enigo::Key::F8,
    enigo::Key::F9,
    enigo::Key::F10,
    enigo::Key::F11,
    enigo::Key::F12,
    enigo::Key::F13,
    enigo::Key::F14,
    enigo::Key::F15,
    enigo::Key::F16,
    enigo::Key::F17,
    enigo::Key::F18,
    enigo::Key::F19,
    enigo::Key::F20,
    enigo::Key::Home,
    enigo::Key::End,
    enigo::Key::PageUp,
    enigo::Key::PageDown,
    enigo::Key::Insert,
    enigo::Key::Delete,
    enigo::Key::Numlock,
    enigo::Key::ScrollLock,
    enigo::Key::Pause,
    enigo::Key::PrintScr,
    enigo::Key::MediaPlayPause,
    enigo::Key::MediaStop,
    enigo::Key::MediaNextTrack,
    enigo::Key::MediaPrevTrack,
    enigo::Key::VolumeUp,
    enigo::Key::VolumeDown,
    enigo::Key::VolumeMute,
];
