use std::ffi::CString;
use std::mem::MaybeUninit;
use std::ptr::{null, null_mut};
use std::thread::sleep;
use std::time::Duration;
use x11_dl::{xinerama, xlib};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Action {
    Run,
    Stop,
}

pub struct X11Context {
    xlib: xlib::Xlib,
    xin: xinerama::Xlib,
    display: *mut xlib::_XDisplay,
    root: u64,
}

pub struct Screens {
    screens: *mut xinerama::XineramaScreenInfo,
    screens_number: i32,
    i: i32,
}

impl Iterator for Screens {
    type Item = xinerama::XineramaScreenInfo;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.screens_number {
            let screen = unsafe { *(self.screens.offset(self.i as isize)) };
            self.i += 1;
            return Some(screen);
        } else {
            return None;
        }
    }
}

impl X11Context {
    pub fn new() -> Self {
        unsafe {
            // Load Xlib library.
            let xlib = xlib::Xlib::open().expect("failed to load xlib");

            let xin = x11_dl::xinerama::Xlib::open().expect("couldn't load xinerama");

            // Open display connection.
            let display = (xlib.XOpenDisplay)(null());

            if display.is_null() {
                panic!("XOpenDisplay failed");
            }

            let screen = (xlib.XDefaultScreen)(display);
            let root = (xlib.XRootWindow)(display, screen);

            Self {
                xlib,
                xin,
                display,
                root,
            }
        }
    }
    pub fn get_screens(&self) -> Screens {
        let mut screens_number = 0;
        let screens = unsafe { (self.xin.XineramaQueryScreens)(self.display, &mut screens_number) };
        Screens {
            screens,
            screens_number,
            i: 0,
        }
    }
    pub fn get_mouse_pos(&self) -> (i32, i32) {
        unsafe {
            let (mut x, mut y) = (0, 0);
            (self.xlib.XQueryPointer)(
                self.display,
                self.root,
                &mut 0,
                &mut 0,
                &mut x,
                &mut y,
                &mut 0,
                &mut 0,
                &mut 0,
            );

            (x, y)
        }
    }
    pub fn create_window(&self, x: i32, y: i32, width: u32, height: u32) -> u64 {
        unsafe {
            let mut attributes: xlib::XSetWindowAttributes = MaybeUninit::zeroed().assume_init();
            attributes.override_redirect = xlib::True;

            (self.xlib.XCreateWindow)(
                self.display,
                self.root,
                x,
                y,
                width,
                height,
                0,
                xlib::CopyFromParent,
                xlib::InputOutput as u32,
                null_mut(),
                xlib::CWOverrideRedirect,
                &mut attributes,
            )
        }
    }
    pub fn map_window(&self, window: u64) {
        unsafe {
            (self.xlib.XMapRaised)(self.display, window);
        }
    }
    pub fn grab_keyboard(&self) {
        for _ in 0..1000 {
            if unsafe {
                (self.xlib.XGrabKeyboard)(
                    self.display,
                    self.root,
                    xlib::True,
                    xlib::GrabModeAsync,
                    xlib::GrabModeAsync,
                    xlib::CurrentTime,
                )
            } == 0
            {
                // Successfully grabbed keyboard
                break;
            } else {
                // Try again
                sleep(Duration::from_millis(1));
            }
        }
    }
    pub fn load_font(&self, font: &str) -> u64 {
        unsafe { (self.xlib.XLoadFont)(self.display, CString::new(font).unwrap().as_ptr()) }
    }
    pub fn init_gc(&self, window: u64, font: u64) -> *mut xlib::_XGC {
        unsafe {
            let mut xgc_values: xlib::XGCValues = MaybeUninit::zeroed().assume_init();
            xgc_values.font = font;
            (self.xlib.XCreateGC)(self.display, window, xlib::GCFont as u64, &mut xgc_values)
        }
    }
    pub fn get_font_size(&self, font: u64) -> i32 {
        unsafe {
            let font_struct = (self.xlib.XQueryFont)(self.display, font);
            ((*font_struct).max_bounds.ascent + (*font_struct).max_bounds.descent) as i32
        }
    }
    pub fn run<F>(&self, mut handle_events: F)
    where
        F: FnMut(&Self, Option<&xlib::XEvent>) -> Action,
    {
        let mut event = MaybeUninit::<xlib::XEvent>::uninit();

        loop {
            if unsafe {
                (self.xlib.XCheckMaskEvent)(self.display, xlib::KeyPressMask, event.as_mut_ptr())
            } == 0
            {
                // no events available
                // execute given closure and wait for the next frame
                if handle_events(&self, None) == Action::Stop {
                    break;
                }

                sleep(Duration::from_nanos(1_000_000_000 / 60));
            } else {
                // we got some events
                if handle_events(&self, Some(unsafe { &event.assume_init() })) == Action::Stop {
                    break;
                }
            }
        }
    }
    pub fn shutdown(self) {
        unsafe {
            (self.xlib.XCloseDisplay)(self.display);
        }
    }
    pub fn draw_rect(
        &self,
        window: u64,
        gc: *mut xlib::_XGC,
        color: u64,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) {
        unsafe {
            (self.xlib.XSetForeground)(self.display, gc, color);
            (self.xlib.XFillRectangle)(self.display, window, gc, x, y, width, height);
        }
    }
    pub fn render_text(
        &self,
        window: u64,
        gc: *mut xlib::_XGC,
        color: u64,
        x: i32,
        y: i32,
        text: &str,
    ) {
        unsafe {
            (self.xlib.XSetForeground)(self.display, gc, color);
            (self.xlib.XDrawString)(
                self.display,
                window,
                gc,
                x,
                y,
                text.as_ptr() as *const i8,
                text.len() as i32,
            );
        }
    }
    pub fn keyevent_to_char(&self, mut keyevent: xlib::XKeyEvent) -> char {
        unsafe {
            let mut c_char: i8 = 0;
            (self.xlib.XLookupString)(&mut keyevent, &mut c_char, 1, null_mut(), null_mut());
            c_char as u8 as char
        }
    }
    pub fn xevent_to_xkeyevent(&self, xevent: xlib::XEvent) -> Option<xlib::XKeyEvent> {
        match xevent.get_type() {
            xlib::KeyPress => Some(unsafe { xevent.key }),
            _ => None,
        }
    }
}
