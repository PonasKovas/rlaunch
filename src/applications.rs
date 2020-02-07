use std::collections::HashMap;
use std::fs::{read_dir, File};
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::env::var;

pub type Apps = HashMap<String, (String, Terminal)>;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Terminal {
    Show,
    Hide,
}

pub fn read_applications(apps: Arc<Mutex<Apps>>) {
    let now = Instant::now();
    // iterate over all files in applications directories
    for dir in &[
        "/usr/share/applications",
        "/usr/local/share/applications",
        &format!("{}/.local/share/applications", var("HOME").unwrap()),
    ] {
        let files_iterator = match read_dir(dir) {
            Ok(iterator) => iterator,
            Err(e) => {
                println!("Couldn't read the files in {} ({})", dir, e);
                continue;
            }
        };
        for file in files_iterator {
            let file = match file {
                Ok(f) => f,
                Err(_) => continue,
            };
            // make sure it's a .desktop file
            let path = file.path();
            let extension = match path.extension() {
                Some(e) => e,
                None => continue,
            };
            if extension != "desktop" {
                continue;
            }

            // read the file contents
            let mut file = match File::open(path) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let mut contents = String::new();
            if let Err(_) = file.read_to_string(&mut contents) {
                continue;
            }
            let mut name = String::new();
            let mut exec = String::new();
            let mut app_type = String::new();
            let mut terminal = String::new();
            for line in contents.split('\n') {
                if exec == "" && line.starts_with("Exec=") {
                    exec = line[5..].to_string();
                    // remove any arguments
                    while let Option::Some(i) = exec.find("%") {
                        exec.replace_range(i..(i+2), "");
                    }
                    // remove quotes if present
                    if exec.len()>1 && exec.starts_with("\"") && exec.ends_with("\"") {
                        exec = exec[1..exec.len()-1].to_string();
                    }
                    exec = exec.trim().to_owned();
                } else if name == "" && line.starts_with("Name=") {
                    name = line[5..].to_string();
                    // remove quotes if present
                    if name.len()>1 && name.starts_with("\"") && name.ends_with("\"") {
                        name = name[1..name.len()-1].to_string();
                    }
                } else if app_type == "" && line.starts_with("Type=") {
                    app_type = line[5..].to_string();
                } else if terminal == "" && line.starts_with("Terminal=") {
                    terminal = line[9..].to_string();
                }
            }
            if name == "" {
                continue;
            }
            if app_type != "Application" {
                continue;
            }
            if exec == "" {
                continue;
            }
            terminal.make_ascii_lowercase();
            let terminal = if terminal == "" || terminal == "false" {
                Terminal::Hide
            } else {
                Terminal::Show
            };
            apps.lock().unwrap().insert(name, (exec, terminal));
        }
    }
    println!(
        "Finished reading all applications ({}s)",
        now.elapsed().as_secs_f64()
    );
}
