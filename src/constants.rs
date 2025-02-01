use crate::utility::EnigoKey;

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

pub const HOME_IMAGE_WIDTH: usize = 42;
pub const HOME_IMAGE_HEIGHT: usize = 42;
pub const HOME_IMAGE_BYTES_SIZE: usize = ((HOME_IMAGE_WIDTH + (8 - 1)) / 8) * HOME_IMAGE_HEIGHT;
pub const HOME_IMAGE_DEFAULT_BYTES: &str = "0000000000FC0000FE0100FC00E0FF1F00FC00F8FF7F00FC00FCFFFF00FC00FFFFFF03FC80FFFFFF07FCC0FF03FF0FFCE0FF00FC1FFCE07F00F81FFCF03F00F03FFCF83F00F07FFCF81F00E07FFCFC1F00E0FFFCFC1F00E0FFFCFC1F00E0FFFCFC1F00E0FFFCFE1F00E0FFFDFE3F00F0FFFDFE3F00F0FFFDFE7F00F8FFFDFEFF00FCFFFDFEFF03FFFFFDFEFFFFFFFFFDFEFFFFFFFFFDFCFFFFFFFFFCFCFF00FCFFFCFC0F00C0FFFCFC010000FEFCF80000007CFC7800000078FC7000000038FCE00000001CFCE00100001EFCC00300000FFC800F00C007FC007F00F803FC00FC03FF00FC00F8FF7F00FC00E0FF1F00FC0000FE0100FC0000000000FC";

// Validation
pub const FORBIDDEN_CHARACTERS: [&str; 3] = [
    SERIAL_MESSAGE_SEP,
    SERIAL_MESSAGE_INNER_SEP,
    SERIAL_MESSAGE_END,
];

// Debug
pub const DEBUG_SERIAL_DISABLE: bool = true; // Disables Serial comm for testing dashboard
pub const DEBUG_TCP_CLIENT_CONNECTION: bool = false;
pub const DEBUG_TCP_SERVER_MESSAGE_CONFIRMATION: bool = false; // Logging received/sent messages

pub static KEYS: [EnigoKey; 87] = [
    EnigoKey(enigo::Key::Control),
    EnigoKey(enigo::Key::Shift),
    EnigoKey(enigo::Key::Alt),
    EnigoKey(enigo::Key::Meta),
    EnigoKey(enigo::Key::Unicode('a')),
    EnigoKey(enigo::Key::Unicode('b')),
    EnigoKey(enigo::Key::Unicode('c')),
    EnigoKey(enigo::Key::Unicode('d')),
    EnigoKey(enigo::Key::Unicode('e')),
    EnigoKey(enigo::Key::Unicode('f')),
    EnigoKey(enigo::Key::Unicode('g')),
    EnigoKey(enigo::Key::Unicode('h')),
    EnigoKey(enigo::Key::Unicode('i')),
    EnigoKey(enigo::Key::Unicode('j')),
    EnigoKey(enigo::Key::Unicode('k')),
    EnigoKey(enigo::Key::Unicode('l')),
    EnigoKey(enigo::Key::Unicode('m')),
    EnigoKey(enigo::Key::Unicode('n')),
    EnigoKey(enigo::Key::Unicode('o')),
    EnigoKey(enigo::Key::Unicode('p')),
    EnigoKey(enigo::Key::Unicode('q')),
    EnigoKey(enigo::Key::Unicode('r')),
    EnigoKey(enigo::Key::Unicode('s')),
    EnigoKey(enigo::Key::Unicode('t')),
    EnigoKey(enigo::Key::Unicode('u')),
    EnigoKey(enigo::Key::Unicode('v')),
    EnigoKey(enigo::Key::Unicode('w')),
    EnigoKey(enigo::Key::Unicode('x')),
    EnigoKey(enigo::Key::Unicode('y')),
    EnigoKey(enigo::Key::Unicode('z')),
    EnigoKey(enigo::Key::Unicode('0')),
    EnigoKey(enigo::Key::Unicode('1')),
    EnigoKey(enigo::Key::Unicode('2')),
    EnigoKey(enigo::Key::Unicode('3')),
    EnigoKey(enigo::Key::Unicode('4')),
    EnigoKey(enigo::Key::Unicode('5')),
    EnigoKey(enigo::Key::Unicode('6')),
    EnigoKey(enigo::Key::Unicode('7')),
    EnigoKey(enigo::Key::Unicode('8')),
    EnigoKey(enigo::Key::Unicode('9')),
    EnigoKey(enigo::Key::Return),
    EnigoKey(enigo::Key::Tab),
    EnigoKey(enigo::Key::Backspace),
    EnigoKey(enigo::Key::Escape),
    EnigoKey(enigo::Key::Space),
    EnigoKey(enigo::Key::CapsLock),
    EnigoKey(enigo::Key::LeftArrow),
    EnigoKey(enigo::Key::RightArrow),
    EnigoKey(enigo::Key::UpArrow),
    EnigoKey(enigo::Key::DownArrow),
    EnigoKey(enigo::Key::F1),
    EnigoKey(enigo::Key::F2),
    EnigoKey(enigo::Key::F3),
    EnigoKey(enigo::Key::F4),
    EnigoKey(enigo::Key::F5),
    EnigoKey(enigo::Key::F6),
    EnigoKey(enigo::Key::F7),
    EnigoKey(enigo::Key::F8),
    EnigoKey(enigo::Key::F9),
    EnigoKey(enigo::Key::F10),
    EnigoKey(enigo::Key::F11),
    EnigoKey(enigo::Key::F12),
    EnigoKey(enigo::Key::F13),
    EnigoKey(enigo::Key::F14),
    EnigoKey(enigo::Key::F15),
    EnigoKey(enigo::Key::F16),
    EnigoKey(enigo::Key::F17),
    EnigoKey(enigo::Key::F18),
    EnigoKey(enigo::Key::F19),
    EnigoKey(enigo::Key::F20),
    EnigoKey(enigo::Key::Home),
    EnigoKey(enigo::Key::End),
    EnigoKey(enigo::Key::PageUp),
    EnigoKey(enigo::Key::PageDown),
    EnigoKey(enigo::Key::Insert),
    EnigoKey(enigo::Key::Delete),
    EnigoKey(enigo::Key::Numlock),
    EnigoKey(enigo::Key::ScrollLock),
    EnigoKey(enigo::Key::Pause),
    EnigoKey(enigo::Key::PrintScr),
    EnigoKey(enigo::Key::MediaPlayPause),
    EnigoKey(enigo::Key::MediaStop),
    EnigoKey(enigo::Key::MediaNextTrack),
    EnigoKey(enigo::Key::MediaPrevTrack),
    EnigoKey(enigo::Key::VolumeUp),
    EnigoKey(enigo::Key::VolumeDown),
    EnigoKey(enigo::Key::VolumeMute),
];
