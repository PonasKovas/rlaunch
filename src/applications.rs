use std::env::var;
use std::fs::{read_dir, read_to_string};
use std::sync::Mutex;
use std::time::Instant;
use std::collections::BTreeMap;

pub type Apps = BTreeMap<String, App>;
type DirID = String;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct App {
    pub exec: String,
    pub show_terminal: bool,
}

pub fn read_applications(apps: &Mutex<Apps>, scan_path: bool, progress: &Mutex<(u32, u32)>) {
    let xdg_data_home = match var("XDG_DATA_HOME") {
        Ok(h) => (h + "/applications", "".to_owned()),
        Err(_) => match var("HOME") {
            Ok(s) => (s + "/.local/share/applications", "".to_owned()),
            Err(_) => {
                eprintln!("$HOME not set!");
                ("".to_owned(), "".to_owned())
            }
        },
    };
    let mut xdg_data_dirs = match var("XDG_DATA_DIRS") {
        Ok(d) => d
            .split(':')
            .map(|s| (s.to_owned() + "/applications", "".to_owned()))
            .collect(),
        Err(_) => "/usr/local/share/:/usr/share/"
            .split(':')
            .map(|s| (s.to_owned() + "/applications", "".to_owned()))
            .collect(),
    };

    // all dirs that might have `applications` dir inside that we need to scan
    let mut share_dirs = vec![xdg_data_home];
    share_dirs.append(&mut xdg_data_dirs);

    // count the files for the progress bar
    let mut files_to_scan = 0;
    let mut i = 0;
    let mut len = share_dirs.len();
    while i < len {
        let files = match read_dir(&share_dirs[i].0) {
            Ok(f) => f,
            Err(_) => {
                share_dirs.remove(i);
                len -= 1;
                continue;
            }
        };
        for file in files {
            match file {
                Ok(f) => {
                    if f.path().is_dir() {
                        share_dirs.insert(
                            i + 1,
                            match f.path().to_str() {
                                Some(s) => (
                                    s.to_owned(),
                                    share_dirs[i].1.to_owned()
                                        + "/"
                                        + f.file_name().to_str().unwrap(),
                                ),
                                None => continue,
                            },
                        );
                    } else {
                        files_to_scan += 1;
                    }
                }
                Err(_) => continue,
            }
        }
        i += 1;
        len = share_dirs.len();
    }

    // and files in $PATH too, if -p flag set
    if scan_path {
        if let Ok(path) = var("PATH") {
            for dir in path.split(':') {
                let files = match read_dir(dir) {
                    Ok(f) => f,
                    Err(_) => {
                        continue;
                    }
                };
                for file in files {
                    match file {
                        Ok(f) => {
                            if f.path().is_dir() {
                                continue;
                            } else {
                                files_to_scan += 1;
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
        }
    }

    // get the progress bar ready
    progress.lock().unwrap().1 = files_to_scan;

    let now = Instant::now();

    // start actually scanning files
    scan_desktop_entries(apps, share_dirs, progress);

    if scan_path {
        scan_path_dirs(apps, progress);
    }

    println!(
        "Finished reading all {} applications ({}s)",
        files_to_scan,
        now.elapsed().as_secs_f64()
    );
}

fn scan_desktop_entries(
    apps: &Mutex<Apps>,
    dirs: Vec<(String, DirID)>,
    progress: &Mutex<(u32, u32)>,
) {
    let mut scanned_ids = Vec::new();
    for dir in dirs {
        println!("scanning {:?}", dir);
        'files: for file in read_dir(dir.0).unwrap() {
            let file = file.unwrap();

            if file.path().is_dir() {
                continue;
            }

            // update progress
            progress.lock().unwrap().0 += 1;

            let path = file.path();

            // if file doesn't end in .desktop, move on
            if !path
                .extension()
                .map(|ext| ext == "desktop")
                .unwrap_or(false)
            {
                continue;
            }

            // get the freedesktop.org file ID
            let mut file_id = String::new();
            if !dir.1.is_empty() {
                file_id += &dir.1[1..].replace('/', "-");
                file_id += "-";
            }
            if let Some(stem) = path.file_stem() {
                file_id += &stem.to_string_lossy();
            };

            // if there were any other files with the same ID before, ignore this file
            if scanned_ids.contains(&file_id) {
                continue;
            }
            scanned_ids.push(file_id);

            // cool. now we can start parsing the file
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
                    let mut value = line[7..].to_string();
                    remove_quotes(&mut value);
                    match value.trim().to_lowercase().parse() {
                        Err(_) | Ok(true) => {
                            // hidden or couldnt parse
                            continue 'files;
                        }
                        _ => {}
                    }
                } else if line.starts_with("NoDisplay=") {
                    let mut value = line[10..].to_string();
                    remove_quotes(&mut value);
                    match value.trim().to_lowercase().parse() {
                        Err(_) | Ok(true) => {
                            // nodisplay or couldnt parse
                            continue 'files;
                        }
                        _ => {}
                    }
                } else if exec == "" && line.starts_with("Exec=") {
                    exec = line[5..].to_string();
                    // remove any arguments
                    while let Some(i) = exec.find('%') {
                        exec.replace_range(i..(i + 2), "");
                    }
                    remove_quotes(&mut exec);
                    exec = exec.trim().to_owned();
                } else if name == "" && line.starts_with("Name=") {
                    name = line[5..].to_string();
                    remove_quotes(&mut name);
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

            apps.lock().unwrap().insert(name, App {
                exec,
                show_terminal: terminal,
            });
        }
    }
}

fn scan_path_dirs(apps: &Mutex<Apps>, progress: &Mutex<(u32, u32)>) {
    if let Ok(path) = var("PATH") {
        for dir in path.split(':') {
            let files = match read_dir(dir) {
                Ok(f) => f,
                Err(_) => {
                    continue;
                }
            };
            for file in files {
                let file = match file {
                    Ok(f) => {
                        if f.path().is_dir() {
                            continue;
                        }
                        f
                    }
                    Err(_) => continue,
                };

                // update progress
                progress.lock().unwrap().0 += 1;

                let path = file.path();
                let name = match path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                {
                    Some(name) => name,
                    None => continue,
                };
                let exec = path.to_string_lossy().into_owned();

                apps.lock().unwrap().insert(name, App {
                    exec,
                    show_terminal: false,
                });
            }
        }
    }
}

fn remove_quotes(string: &mut String) {
    // remove quotes if present
    if string.len() > 1 && string.starts_with('"') && string.ends_with('"') {
        *string = string[1..string.len() - 1].to_string();
    }
}
