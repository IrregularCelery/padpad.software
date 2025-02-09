use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::Path,
    sync::{Mutex, OnceLock},
};

use crate::{constants::APP_NAME, utility::get_app_name_without_extension};

static LOGGER: OnceLock<Logger> = OnceLock::new();

pub struct Logger {
    file: Mutex<std::fs::File>,
}

pub fn get_logger() -> &'static Logger {
    init_logger()
}

impl Logger {
    fn new(log_path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(log_path)?;
        Ok(Logger {
            file: Mutex::new(file),
        })
    }

    pub fn log(
        &self,
        level: &str,
        message: &str,
        #[allow(unused_variables)] trace_string: String,
        write_to_file: bool,
    ) {
        let timestamp = chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S.%f")
            .to_string();
        let thread_id = std::thread::current();
        #[allow(unused_mut)]
        let mut trace = String::new();

        #[cfg(debug_assertions)]
        if !trace_string.is_empty() {
            trace = format!("[{}]", trace_string);
        }

        let log_message = format!(
            "[ {} ][{}][{}][{}]{} {}",
            level,
            timestamp,
            if let Some(app_name) = get_app_name_without_extension() {
                app_name
            } else {
                APP_NAME.to_string()
            },
            thread_id.name().unwrap_or("Unknown"),
            trace,
            message,
        );

        // Write to the log file
        let mut file = self.file.lock().unwrap();

        if write_to_file {
            writeln!(file, "{}", log_message).unwrap();
        }

        // Print to console
        println!("{}", log_message);
    }
}

fn init_logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        let app_path = std::env::current_exe().expect("Failed to get current exe path");
        let log_path = app_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!("{}.log", APP_NAME.to_lowercase()));

        Logger::new(&log_path).expect("Failed to initialize logger")
    })
}

// Logging Macros
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::log::get_logger().log(
            "INFO ",
            &format!($($arg)*),
            String::new(),
            true
        )
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::log::get_logger().log(
            "WARN ",
            &format!($($arg)*),
            String::new(),
            true
        );
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::log::get_logger().log(
            "ERROR",
            &format!($($arg)*),
            String::new(),
            true
        )
    };
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        $crate::log::get_logger().log(
            "TRACE",
            &format!($($arg)*),
            format!("{}:{}:{}",
            file!(),
            line!(),
            std::any::type_name::<fn()>()),
            true
        );
    };
}

#[macro_export]
macro_rules! log_print {
    ($($arg:tt)*) => {
        $crate::log::get_logger().log(
            "PRINT",
            &format!($($arg)*),
            String::new(),
            false
        )
    };
}
