use std::ffi::CString;
use std::mem::MaybeUninit;
use std::ptr::{null, null_mut};
use std::thread::sleep;
use std::time::Duration;
use x11_dl::{xft, xinerama, xlib};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Action {
    Run,
    Stop,
}

pub struct X11Context {
    xlib: xlib::Xlib,
    xin: xinerama::Xlib,
    xft: xft::Xft,
    display: *mut xlib::_XDisplay,
    root: u64,
}

pub struct TextRenderingContext {
    visual: *mut xlib::Visual,
    cmap: u64,
    font: *mut xft::XftFont,
    colors: Vec<xft::XftColor>,
    draw: *mut xft::XftDraw,
}

pub struct GraphicsContext {
    gc: *mut xlib::_XGC,
    window: u64,
}

pub struct Window {
    window: u64,
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
            Some(screen)
        } else {
            None
        }
    }
}

impl X11Context {
    pub fn new() -> Result<Self, &'static str> {
        unsafe {
            // Load Xlib library.
            let xlib = xlib::Xlib::open().map_err(|_| "Failed to load XLib.")?;

            // load xinerama
            let xin = xinerama::Xlib::open().map_err(|_| "Failed to load Xinerama.")?;

            // load xft
            let xft = xft::Xft::open().map_err(|_| "Failed to load XFT")?;

            // Open display connection.
            let display = (xlib.XOpenDisplay)(null());

            if display.is_null() {
                return Err("XOpenDisplay failed");
            }

            let screen = (xlib.XDefaultScreen)(display);
            let root = (xlib.XRootWindow)(display, screen);

            Ok(Self {
                xlib,
                xin,
                xft,
                display,
                root,
            })
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
    pub fn create_window(&self, pos: (i32, i32), width: u32, height: u32) -> Window {
        unsafe {
            let mut attributes: xlib::XSetWindowAttributes = MaybeUninit::zeroed().assume_init();
            attributes.override_redirect = xlib::True;

            Window {
                window: (self.xlib.XCreateWindow)(
                    self.display,
                    self.root,
                    pos.0,
                    pos.1,
                    width,
                    height,
                    0,
                    xlib::CopyFromParent,
                    xlib::InputOutput as u32,
                    null_mut(),
                    xlib::CWOverrideRedirect,
                    &mut attributes,
                ),
            }
        }
    }
    pub fn map_window(&self, window: &Window) {
        unsafe {
            (self.xlib.XMapRaised)(self.display, window.window);
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
    pub fn init_trc(&self, window: &Window, font: &str) -> TextRenderingContext {
        unsafe {
            let cfontname = CString::new(font).unwrap();

            let screen = (self.xlib.XDefaultScreen)(self.display);
            let visual = (self.xlib.XDefaultVisual)(self.display, screen);
            let cmap = (self.xlib.XDefaultColormap)(self.display, screen);

            let font = (self.xft.XftFontOpenName)(self.display, screen, cfontname.as_ptr());
            let colors = Vec::new();

            let draw = (self.xft.XftDrawCreate)(self.display, window.window, visual, cmap);
            TextRenderingContext {
                visual,
                cmap,
                font,
                colors,
                draw,
            }
        }
    }
    /// returns the index to use as the color argument in xc::render_text
    pub fn add_color_to_trc(&self, trc: &mut TextRenderingContext, color: u64) -> usize {
        unsafe {
            let color = CString::new(format!("#{:06X}", color)).unwrap();
            let mut xftcolor: xft::XftColor = MaybeUninit::zeroed().assume_init();
            (self.xft.XftColorAllocName)(
                self.display,
                trc.visual,
                trc.cmap,
                color.as_ptr(),
                &mut xftcolor,
            );

            let index = trc.colors.len();
            trc.colors.push(xftcolor);
            index
        }
    }
    pub fn init_gc(&self, window: &Window) -> GraphicsContext {
        unsafe {
            let mut xgc_values: xlib::XGCValues = MaybeUninit::zeroed().assume_init();
            GraphicsContext {
                gc: (self.xlib.XCreateGC)(self.display, window.window, 0, &mut xgc_values),
                window: window.window,
            }
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
    pub fn draw_rect(
        &self,
        gc: &GraphicsContext,
        color: u64,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) {
        unsafe {
            (self.xlib.XSetForeground)(self.display, gc.gc, color);
            (self.xlib.XFillRectangle)(self.display, gc.window, gc.gc, x, y, width, height);
        }
    }
    pub fn render_text(
        &self,
        trc: &TextRenderingContext,
        color: usize,
        x: i32,
        y: i32,
        text: &str,
    ) {
        unsafe {
            let ctext = CString::new(text).unwrap();

            // render the text
            let mut col = trc.colors[color];
            (self.xft.XftDrawStringUtf8)(
                trc.draw,
                &mut col,
                trc.font,
                x,
                y,
                ctext.as_ptr() as *mut u8,
                text.len() as i32,
            );
        }
    }
    pub fn get_text_dimensions(&self, trc: &TextRenderingContext, text: &str) -> (u16, u16) {
        unsafe {
            // Some fonts treat a single space at the end weirdly
            // which makes typing a bit confusing, so we will add a '/'
            // to the end and then remove it's width from the total width
            let dot = CString::new("/").unwrap();
            let owned_text = text.to_owned();
            let ctext = CString::new(owned_text + "/").unwrap();

            let mut total_ext = MaybeUninit::zeroed().assume_init();
            (self.xft.XftTextExtentsUtf8)(
                self.display,
                trc.font,
                ctext.as_ptr() as *mut u8,
                text.len() as i32 + 1,
                &mut total_ext,
            );
            let mut dot_ext = MaybeUninit::zeroed().assume_init();
            (self.xft.XftTextExtentsUtf8)(
                self.display,
                trc.font,
                dot.as_ptr() as *mut u8,
                1,
                &mut dot_ext,
            );

            (total_ext.width - dot_ext.width, total_ext.height)
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

impl Drop for X11Context {
    fn drop(&mut self) {
        unsafe {
            (self.xlib.XCloseDisplay)(self.display);
        }
    }
}
