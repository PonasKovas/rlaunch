use structopt::StructOpt;

#[derive(Debug, StructOpt, Clone)]
#[structopt(
    name = "rlaunch",
    about = "A simple, light-weight and modern tool for launching applications and running commands on X11."
)]
pub struct Args {
    /// The color of the bar background
    #[structopt(long, default_value = "#2e2c2c", parse(try_from_str = parse_color))]
    pub color0: u64,

    /// The color of the selected suggestion background
    #[structopt(long, default_value = "#1286a1", parse(try_from_str = parse_color))]
    pub color1: u64,

    /// The color of the text
    #[structopt(long, default_value = "#ffffff", parse(try_from_str = parse_color))]
    pub color2: u64,

    /// The color of the suggestions text
    #[structopt(long, default_value = "#ffffff", parse(try_from_str = parse_color))]
    pub color3: u64,

    /// The color of the file scanning progress bar
    #[structopt(long, default_value = "#242222", parse(try_from_str = parse_color))]
    pub color4: u64,

    /// The height of the bar (in pixels)
    #[structopt(short, long, default_value = "22")]
    pub height: u32,

    /// Show the bar on the bottom of the screen
    #[structopt(short, long)]
    pub bottom: bool,

    /// The font used on the bar
    #[structopt(short, long, default_value = "DejaVu Sans Mono")]
    pub font: String,

    /// The terminal to use when launching applications that require a terminal
    #[structopt(short, long, default_value = "i3-sensible-terminal")]
    pub terminal: String,

    /// Scan the PATH variable.
    #[structopt(short, long)]
    pub path: bool,
}

pub fn get_args() -> Args {
    Args::from_args()
}

fn parse_color(string: &str) -> Result<u64, &str> {
    if !string.starts_with('#') {
        return Err("Color hex code must start with a #");
    }
    if string.len() != 7 {
        return Err("Color hex code format: #RRGGBB");
    }

    u64::from_str_radix(&string[1..], 16).map_err(|_| "Couldn't parse color code")
}
