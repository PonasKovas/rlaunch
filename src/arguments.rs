use structopt::StructOpt;

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "rlaunch", about = "A simple and light-weight tool for launching applications and running commands on X11.")]
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

    /// The height of the bar (in pixels)
    #[structopt(short, long, default_value = "22")]
    pub height: u32,

    /// Show the bar on the bottom of the screen
    #[structopt(short, long)]
    pub bottom: bool,

    /// The font used on the bar
    /// Use `xfontsel` to determine it
    #[structopt(short, long, default_value = "-*-fixed-medium-*-*-*-18-*-*-*-*-*-*-*")]
    pub font: String,

    /// The terminal to use when launching applications that require a terminal
    #[structopt(short, long, default_value = "i3-sensible-terminal")]
    pub terminal: String,
}

pub fn get_args() -> Args {
    Args::from_args()
}

fn parse_color(string: &str) -> Result<u64, &str> {
    if !string.starts_with("#") {
        return Err("Color hex code must start with a #");
    }
    if string.len() != 7 {
        return Err("Color hex code format: #RRGGBB");
    }
    let mut color: u64 = 0;
    for (i, c) in string[1..].as_bytes().chunks(2).enumerate() {
        let hex = String::from_utf8_lossy(c);
        let col = match u64::from_str_radix(&hex, 16) {
            Ok(x) => x,
            Err(_)=> return Err("Couldn't parse color code"),
        };
        color += col << (2-i)*8;
    }
    Ok(color)
}
