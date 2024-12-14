use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};
use open;
use std::io::{self, Write};
use std::process::Command;

fn main() {
    loop {
        println!("Select action:");
        println!("1) Run a command");
        println!("2) Open an application");
        println!("3) Open a website");
        println!("4) Simulate a keyboard shortcut");
        println!("5) Open a file");
        println!("6) Exit");

        print!("Enter your choice: ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        io::stdin()
            .read_line(&mut choice)
            .expect("Failed to read input");
        let choice = choice.trim();

        match choice {
            "1" => {
                print!("Enter the command to run: ");

                io::stdout().flush().unwrap();
                let mut command = String::new();
                let shell = "sh";
                io::stdin()
                    .read_line(&mut command)
                    .expect("Failed to read input");

                run_command(command.trim(), shell);
            }
            "2" => {
                print!("Enter the application name to open: ");

                io::stdout().flush().unwrap();
                let mut app = String::new();
                io::stdin()
                    .read_line(&mut app)
                    .expect("Failed to read input");

                open_application(app.trim());
            }
            "3" => {
                print!("Enter the URL to open: ");

                io::stdout().flush().unwrap();
                let mut url = String::new();
                io::stdin()
                    .read_line(&mut url)
                    .expect("Failed to read input");

                open_website(url.trim());
            }
            "4" => {
                print!("Enter the shortcut to simulate (e.g., ctrl+c): ");

                io::stdout().flush().unwrap();
                let mut shortcut = String::new();
                io::stdin()
                    .read_line(&mut shortcut)
                    .expect("Failed to read input");

                simulate_shortcut(shortcut.trim());
            }
            "5" => {
                print!("Enter the file's full path to open: ");
                io::stdout().flush().unwrap();
                let mut file_path = String::new();
                io::stdin()
                    .read_line(&mut file_path)
                    .expect("Failed to read input");

                open_file(file_path.trim());
            }
            "6" => {
                println!("Goodbye!");

                break;
            }
            _ => println!("Invalid choice. Please try again."),
        }
    }
}

fn run_command(command: &str, unix_shell: &str) {
    let cmd = command.trim();

    if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", cmd])
            .spawn()
            .expect("Failed to run command");
    } else {
        let shell = unix_shell.trim();

        Command::new(shell)
            .arg("-c")
            .arg(cmd)
            .spawn()
            .expect("Failed to run command");
    }

    println!("Command executed: {}", cmd);
}

fn open_application(app_full_path: &str) {
    let app_path = app_full_path.trim();

    Command::new(app_path)
        .spawn()
        .expect("Failed to open application");

    println!("Application opened: {}", app_path);
}

fn open_website(website_url: &str) {
    let url = website_url.trim();

    let full_url = if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{}", url)
    };

    open::that_detached(&full_url).expect("Failed to open website");

    println!("Website opened: {}", full_url);
}

fn simulate_shortcut(shortcut: &str) {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();

    match shortcut {
        "test" => {
            enigo
                .text("Subscribe to IrregularCelery on YouTube please :D")
                .unwrap();
        }
        "dmenu" => {
            enigo.key(Key::Alt, Press).unwrap();
            enigo.key(Key::Unicode('p'), Click).unwrap();
            enigo.key(Key::Alt, Release).unwrap();
        }
        _ => println!("Shortcut not recognized or not implemented."),
    }
}

fn open_file(file_full_path: &str) {
    let file_path = file_full_path.trim();

    open::that_detached(&file_path).expect("Failed to open file");

    println!("File opened: {}", file_path);
}
