use std::env::var;
use std::fs::{read_dir, read_to_string};
use std::sync::Mutex;
use std::time::Instant;

pub type Apps = Vec<App>;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct App {
    pub name: String,
    pub exec: String,
    pub show_terminal: bool,
}

fn do_read_applications(
    apps: &Mutex<Apps>,
    dirs: &[&str],
    nondesktop: bool,
    progress: &Mutex<(u32, u32)>,
) {
    // get a vector of all files so we can track progress easily
    let mut files = Vec::new();
    for dir in dirs {
        let files_iterator = match read_dir(dir) {
            Ok(iterator) => iterator,
            Err(e) => {
                eprintln!("Couldn't read the files in {} ({})", dir, e);
                continue;
            }
        };
        files.append(&mut files_iterator.collect());
    }

    progress.lock().unwrap().1 = files.len() as u32;

    'files: for file in files {
        // update progress
        progress.lock().unwrap().0 += 1;

        let file = match file {
            Ok(f) => f,
            Err(_) => continue,
        };

        let path = file.path();
        let (name, exec, terminal) = if path
            .extension()
            .map(|extension| extension == "desktop")
            .unwrap_or(false)
        {
            let contents = match read_to_string(path) {
                Ok(contents) => contents,
                Err(_) => continue,
            };

            let mut name = String::new();
            let mut exec = String::new();
            let mut app_type = String::new();
            let mut terminal = String::new();
            for line in contents.lines() {
                if line.starts_with("Hidden=") {
                    let mut hidden = line[7..].to_string();
                    // remove quotes if present
                    if hidden.len() > 1 && hidden.starts_with('"') && hidden.ends_with('"') {
                        hidden = hidden[1..hidden.len() - 1].to_string();
                    }
                    match hidden.trim().to_lowercase().parse() {
                        Err(_) | Ok(true) => { // hidden or couldnt parse
                            continue 'files;
                        },
                        _ => {},
                    }
                } else if exec == "" && line.starts_with("Exec=") {
                    exec = line[5..].to_string();
                    // remove any arguments
                    while let Some(i) = exec.find('%') {
                        exec.replace_range(i..(i + 2), "");
                    }
                    // remove quotes if present
                    if exec.len() > 1 && exec.starts_with('"') && exec.ends_with('"') {
                        exec = exec[1..exec.len() - 1].to_string();
                    }
                    exec = exec.trim().to_owned();
                } else if name == "" && line.starts_with("Name=") {
                    name = line[5..].to_string();
                    // remove quotes if present
                    if name.len() > 1 && name.starts_with('"') && name.ends_with('"') {
                        name = name[1..name.len() - 1].to_string();
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
            let terminal = !(terminal == "" || terminal == "false");

            (name, exec, terminal)
        } else {
            if !nondesktop {
                continue;
            }

            let name = match path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
            {
                Some(name) => name,
                None => continue,
            };
            let exec = path.to_string_lossy().into_owned();

            (name, exec, false)
        };

        apps.lock().unwrap().push(App {
            name,
            exec,
            show_terminal: terminal,
        });
    }

    // sort the apps alphabetically
    apps.lock().unwrap().sort_unstable();
}

pub fn read_applications(apps: &Mutex<Apps>, scan_path: bool, progress: &Mutex<(u32, u32)>) {
    let dirs: &[&str] = &[
        "/usr/share/applications",
        "/usr/local/share/applications",
        &format!("{}/.local/share/applications", var("HOME").unwrap()),
    ];

    let now = Instant::now();

    if scan_path {
        let path_dirs = var("PATH").unwrap();
        let all_dirs: Vec<&str> = dirs.iter().copied().chain(path_dirs.split(':')).collect();
        do_read_applications(apps, &all_dirs, true, progress);
    } else {
        do_read_applications(apps, dirs, false, progress);
    }

    println!(
        "Finished reading all applications ({}s)",
        now.elapsed().as_secs_f64()
    );
}
