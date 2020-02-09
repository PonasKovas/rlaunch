use std::cmp::{max, min};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use x11_dl::xlib;

mod applications;
mod arguments;
mod x11;

fn main() {
    let args = arguments::get_args();
    // spawn a thread for reading all applications
    let apps = Arc::new(Mutex::new(applications::Apps::new()));
    let apps_clone = apps.clone();
    let path = args.path;
    thread::spawn(move || applications::read_applications(apps_clone, path));

    let mut caret_pos = 0;
    let mut text = String::new();
    let mut suggestions = Vec::<(String, usize)>::new();
    let mut selected = 0;

    // initialize xlib context
    let xc = x11::X11Context::new();

    // get screen width and the position where to map window
    let mut screen_width = 0;
    let mut window_pos = (0, 0);

    let mouse_pos = xc.get_mouse_pos();
    for screen in xc.get_screens() {
        // multiple monitors support
        if in_rect((mouse_pos.0, mouse_pos.1), (screen.x_org, screen.y_org), (screen.width, screen.height)) {
            screen_width = screen.width as u32;
            window_pos.0 = screen.x_org as i32;
            window_pos.1 = if args.bottom {
                screen.y_org as i32 + screen.height as i32 - args.height as i32
            } else {
                screen.y_org as i32
            };
            break;
        }
    }

    // create the window
    let window = xc.create_window(window_pos.0, window_pos.1, screen_width, args.height);

    xc.grab_keyboard();

    let font = xc.load_font(&args.font);
    let font_size = xc.get_font_size(font);

    let gc = xc.init_gc(window, font);

    // show window
    xc.map_window(window);

    xc.run(|xc, event| {
        update_suggestions(
            &mut suggestions,
            screen_width,
            font_size,
            &text,
            &apps,
        );
        render_bar(
            &xc,
            window,
            gc,
            screen_width,
            font_size,
            &text,
            caret_pos,
            &suggestions,
            selected,
            &args,
        );
        match event {
            None => x11::Action::Run,
            Some(e) => {
                handle_event(&xc, e, &mut selected, &mut caret_pos, &mut text, &suggestions, &apps, &args.terminal)
            },
        }
    });

    xc.shutdown();
}

fn render_bar(
        xc: &x11::X11Context,
        window: u64,
        gc: *mut xlib::_XGC,
        width: u32,
        font_size: i32,
        text: &str,
        caret_pos: i32,
        suggestions: &Vec<(String, usize)>,
        selected: u8,
        args: &arguments::Args
    ) {
        let char_width = font_size / 2;
        // clear
        xc.draw_rect(window, gc, args.color0, 0, 0, width, args.height);

        // render the typed text
        let text_y = args.height as i32 / 2 + font_size / 4;
        xc.render_text(window, gc, args.color2, 0, text_y, text);
        // and the caret
        xc.draw_rect(window, gc, args.color2, caret_pos * char_width, 2, 2, args.height - 4);

        // render suggestions
        let mut x = (width as f32 * 0.3).floor() as i32;
        for i in 0..suggestions.len() {
            let name = &suggestions[i].0;
            let name_width = (name.len() + 2) as i32 * char_width;
            // if selected, render rectangle below
            if selected as usize == i {
                xc.draw_rect(window, gc, args.color1, x, 0, name_width as u32, args.height);
            }

            xc.render_text(window, gc, args.color3, x + char_width, text_y, name);

            x += name_width;
        }
}

fn update_suggestions(suggestions: &mut Vec<(String, usize)>, width: u32, font_size: i32, text: &str, apps: &Arc<Mutex<applications::Apps>>) {
    let char_width = font_size / 2;

    suggestions.clear();
    // iterate over application names
    // and find those that contain the typed text
    let mut x = 0;
    let max_width = (width as f32 * 0.7).floor() as i32;
    let apps_lock = apps.lock().unwrap();
    for i in 0..(*apps_lock).len() {
        let name = &apps_lock[i].name;
        if name.to_lowercase().contains(&text.to_lowercase()) {
            if x + (name.len() as i32 + 2) * char_width <= max_width {
                x += char_width * (name.len() as i32 + 2);
                suggestions.push((apps_lock[i].name.clone(), i));
            } else {
                break;
            }
        }
    }
}

fn handle_event(xc: &x11::X11Context, event: &xlib::XEvent, selected: &mut u8, caret_pos: &mut i32, text: &mut String, suggestions: &Vec<(String, usize)>, apps: &Arc<Mutex<applications::Apps>>, terminal: &str) -> x11::Action {
    match xc.xevent_to_xkeyevent(*event) {
        Some(e) => {
            if e.keycode == 9 { // escape
                return x11::Action::Stop;
            } else if e.keycode == 113 { // left arrow
                if *selected == 0 {
                    *caret_pos = max(0, *caret_pos - 1);
                } else {
                    *selected -= 1;
                }
            } else if e.keycode == 114 { // right arrow
                if *caret_pos == text.len() as i32 {
                    *selected = min(*selected + 1, suggestions.len() as u8 - 1);
                } else {
                    *caret_pos += 1;
                }
            } else if e.keycode == 22 { // backspace
                if *caret_pos != 0 {
                    text.remove(*caret_pos as usize - 1);
                    *caret_pos -= 1;
                }
            } else if e.keycode == 36 { // enter
                // if no suggestions available, just run the text, otherwise launch selected application
                if suggestions.len() == 0 {
                    run_command(&format!("{}", text));
                } else {
                    let app = &apps.lock().unwrap()[suggestions[*selected as usize].1];
                    if app.show_terminal {
                        run_command(&format!("{} -e \"{}\"", terminal, app.exec));
                    } else {
                        run_command(&format!("{}", app.exec));
                    }
                }
                return x11::Action::Stop;
            } else if e.keycode == 23 { // tab
                if suggestions.len() != 0 {
                    *text = suggestions[*selected as usize].0.to_string();
                    *caret_pos = text.len() as i32;
                    *selected = 0;
                }
            } else { // some other key
                // try to interpret the key as a character
                let c = xc.keyevent_to_char(e);
                if !c.is_ascii_control() {
                    text.push(c);
                    *caret_pos += 1;
                    *selected = 0;
                }
            }
        },
        None => {},
    }
    match event.get_type() {
        xlib::KeyPress => {

        }
        _ => (),
    }
    x11::Action::Run
}

fn run_command(command: &str) {
    let mut parts = command.split(" ");
    if command.len() != 0 {
        let mut c = Command::new(parts.next().unwrap());
        for arg in parts {
            c.arg(arg);
        }
        let _ = c.spawn();
    }
}

fn in_rect(point: (i32, i32), rect: (i16, i16), rect_size: (i16, i16)) -> bool {
    if point.0 >= rect.0 as i32 && point.0 <= (rect.0+rect_size.0) as i32 {
        if point.1 >= rect.1 as i32 && point.1 <= (rect.1+rect_size.1) as i32 {
            return true;
        }
    }
    false
}
