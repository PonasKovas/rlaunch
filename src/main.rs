use std::cmp::{max, min};
use std::convert::TryInto;
use std::ffi::CString;
use std::mem;
use std::os::raw::*;
use std::process::Command;
use std::ptr;
use std::ptr::null_mut;
use std::sync::{Arc, Mutex};
use std::thread::{self, sleep};
use std::time::Duration;
use x11_dl::xlib;

mod applications;
mod arguments;

fn main() {
    let args = arguments::get_args();
    // spawn a thread for reading all applications
    let apps = Arc::new(Mutex::new(applications::Apps::new()));
    let apps_clone = apps.clone();
    let path = args.path;
    thread::spawn(move || applications::read_applications(apps_clone, path));

    let mut cursor_pos = 0;
    let mut text = String::new();
    let mut suggestions = Vec::<(String, usize)>::new();
    let mut selected = 0;
    unsafe {
        // Load Xlib library.
        let xlib = xlib::Xlib::open().unwrap();

        // Open display connection.
        let display = (xlib.XOpenDisplay)(ptr::null());

        if display.is_null() {
            panic!("XOpenDisplay failed");
        }

        // Create window.
        let screen = (xlib.XDefaultScreen)(display);
        let root = (xlib.XRootWindow)(display, screen);

        let screen_width: u32 = (xlib.XDisplayWidth)(display, screen).try_into().unwrap();
        let screen_height: u32 = (xlib.XDisplayHeight)(display, screen).try_into().unwrap();

        let window_y: i32 = if args.bottom {
            (screen_height - args.height) as i32
        } else {
            0
        };

        let mut attributes: xlib::XSetWindowAttributes = mem::MaybeUninit::uninit().assume_init();
        attributes.background_pixel = args.color0;
        attributes.override_redirect = xlib::True;

        let window = (xlib.XCreateWindow)(
            display,
            root,
            0,
            window_y,
            screen_width,
            args.height,
            0,
            xlib::CopyFromParent,
            xlib::InputOutput as c_uint,
            ptr::null_mut(),
            xlib::CWBackPixel | xlib::CWOverrideRedirect,
            &mut attributes,
        );

        // Grab the keyboard
        for _ in 0..1000 {
            if (xlib.XGrabKeyboard)(
                display,
                root,
                xlib::True,
                xlib::GrabModeAsync,
                xlib::GrabModeAsync,
                xlib::CurrentTime,
            ) == 0
            {
                // Successfully grabbed keyboard
                break;
            } else {
                // Try again
                sleep(Duration::from_nanos(1_000_000)); // 1 millisecond
            }
        }

        // initialize graphics context
        let mut xgc_values: xlib::XGCValues = mem::MaybeUninit::uninit().assume_init();
        xgc_values.font =
            (xlib.XLoadFont)(display, CString::new(args.font.clone()).unwrap().as_ptr());
        let gc = (xlib.XCreateGC)(display, window, xlib::GCFont as u64, &mut xgc_values);

        let font_size = {
            let font_struct = (xlib.XQueryFont)(display, xgc_values.font);
            (*font_struct).max_bounds.ascent + (*font_struct).max_bounds.descent
        };

        // Show window and raise to the top
        (xlib.XMapRaised)(display, window);

        // Main loop.
        let mut event: xlib::XEvent = mem::MaybeUninit::uninit().assume_init();

        loop {
            let suggestions_to_fit = update_suggestions(
                screen_width,
                font_size as u32,
                &text,
                &mut suggestions,
                &apps,
            );
            render_bar(
                &xlib,
                display,
                window,
                gc,
                screen_width,
                font_size as i32,
                &text,
                cursor_pos,
                &suggestions,
                suggestions_to_fit,
                selected,
                &args,
            );

            if (xlib.XCheckMaskEvent)(display, xlib::KeyPressMask, &mut event) == 0 {
                // no events available
                sleep(Duration::from_nanos(1_000_000_000 / 60));
            } else {
                match event.get_type() {
                    xlib::KeyPress => {
                        if event.key.keycode == 9 {
                            // escape
                            break;
                        } else if event.key.keycode == 113 {
                            // left arrow
                            if selected == 0 {
                                cursor_pos = max(0, cursor_pos - 1);
                            } else {
                                selected -= 1;
                            }
                        } else if event.key.keycode == 114 {
                            // right arrow
                            if cursor_pos == text.len() as i32 {
                                selected = min(selected + 1, suggestions_to_fit - 1);
                            } else {
                                cursor_pos += 1;
                            }
                        } else if event.key.keycode == 22 {
                            // backspace
                            if cursor_pos != 0 {
                                text.remove(cursor_pos as usize - 1);
                                cursor_pos -= 1;
                            }
                        } else if event.key.keycode == 36 {
                            // enter
                            // if no suggestions available, just run the text, otherwise launch selected application
                            if suggestions.len() == 0 {
                                run_command(&format!("{}", text));
                            } else {
                                let app = &apps.lock().unwrap()[suggestions[selected as usize].1];
                                if app.show_terminal {
                                    run_command(&format!("{} -e \"{}\"", args.terminal, app.exec));
                                } else {
                                    run_command(&format!("{}", app.exec));
                                }
                            }
                            break;
                        } else if event.key.keycode == 23 {
                            // tab
                            if suggestions.len() != 0 {
                                text = suggestions[selected as usize].0.to_string();
                                cursor_pos = text.len() as i32;
                                selected = 0;
                            }
                        } else {
                            let mut cs: i8 = 0;
                            (xlib.XLookupString)(
                                &mut event.key as *mut xlib::XKeyEvent,
                                &mut cs as *mut i8,
                                1,
                                null_mut(),
                                null_mut(),
                            );
                            let c = cs as u8 as char;
                            if !c.is_ascii_control() {
                                text.push(cs as u8 as char);
                                cursor_pos += 1;
                                selected = 0;
                            }
                        }
                    }

                    _ => (),
                }
            }
        }

        // Shut down.
        (xlib.XCloseDisplay)(display);
    }
}

unsafe fn render_bar(
    xlib: &xlib::Xlib,
    display: *mut xlib::_XDisplay,
    window: xlib::Window,
    gc: xlib::GC,
    screen_width: u32,
    font_size: i32,
    text: &str,
    cursor_pos: i32,
    suggestions: &Vec<(String, usize)>,
    suggestions_to_fit: u8,
    selected: u8,
    args: &arguments::Args,
) {
    // clear
    (xlib.XSetForeground)(display, gc, args.color0);
    (xlib.XFillRectangle)(display, window, gc, 0, 0, screen_width, args.height);

    let text_y = args.height as i32 / 2 + font_size / 4;

    // render the text
    (xlib.XSetForeground)(display, gc, args.color2);
    (xlib.XDrawString)(
        display,
        window,
        gc,
        2,
        text_y,
        text.as_ptr() as *const i8,
        text.len() as i32,
    );
    (xlib.XFillRectangle)(display, window, gc, cursor_pos * 9, 2, 2, args.height - 4); // caret

    // render suggestions
    let mut x = (screen_width as f32 * 0.3).floor() as u32;
    for i in 0..suggestions_to_fit {
        let name = &suggestions[i as usize].0;
        let width = (name.len() + 2) as u32 * 9;
        // if selected, render rectangle below
        if selected == i {
            (xlib.XSetForeground)(display, gc, args.color1);
            (xlib.XFillRectangle)(display, window, gc, x as i32, 0, width, args.height);
        }
        // render text
        (xlib.XSetForeground)(display, gc, args.color3);
        (xlib.XDrawString)(
            display,
            window,
            gc,
            x as i32 + 9,
            text_y,
            name.as_ptr() as *const i8,
            name.len() as i32,
        );
        x += width
    }
}

fn update_suggestions(
    screen_width: u32,
    font_size: u32,
    text: &str,
    suggestions: &mut Vec<(String, usize)>,
    apps: &Arc<Mutex<applications::Apps>>,
) -> u8 {
    let char_width = font_size / 2;

    suggestions.clear();
    // iterate over all application names
    let apps_lock = apps.lock().unwrap();
    for i in 0..(*apps_lock).len() {
        if apps_lock[i].name.to_lowercase().contains(&text.to_lowercase()) {
            suggestions.push((apps_lock[i].name.clone(), i));
        }
    }

    // sort the suggestions alphabetically
    suggestions.sort_unstable();

    let mut suggestions_to_fit = 0;
    let mut x = (screen_width as f32 * 0.3).floor() as u32;
    for (suggestion, _) in suggestions {
        if x + (suggestion.len() as u32 + 2) * char_width <= screen_width {
            x += char_width * (suggestion.len() as u32 + 2);
            suggestions_to_fit += 1;
        } else {
            break;
        }
    }

    suggestions_to_fit
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
