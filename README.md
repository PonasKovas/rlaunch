

# rlaunch ![Rust](https://github.com/PonasKovas/rlaunch/workflows/Rust/badge.svg?branch=actions) ![GitHub](https://img.shields.io/github/license/PonasKovas/rlaunch) ![AUR version](https://img.shields.io/aur/version/rlaunch)

rlaunch is a fast, light-weight and modern application launcher for X11 written in Rust that I made because `dmenu` was too slow for me. I'm glad to say that indeed rlaunch works a lot faster than `dmenu` (at least for me, I haven't tested it on other computers).

![demo](https://i.imgur.com/vOMr0Ci.gif)

## Getting Started

This should work on all linux distributions and DEs that use X11, but if it doesn't - feel free to file an issue.

### Usage

```
rlaunch 1.3.13
A simple and light-weight tool for launching applications and running commands on X11.

USAGE:
    rlaunch [FLAGS] [OPTIONS]

FLAGS:
    -b, --bottom     Show the bar on the bottom of the screen
        --help       Prints help information
    -p, --path       Scan the PATH variable
    -V, --version    Prints version information

OPTIONS:
        --color0 <color0>        The color of the bar background [default: #2e2c2c]
        --color1 <color1>        The color of the selected suggestion background [default: #1286a1]
        --color2 <color2>        The color of the text [default: #ffffff]
        --color3 <color3>        The color of the suggestions text [default: #ffffff]
        --color4 <color4>        The color of the file scanning progress bar [default: #242222]
    -f, --font <font>            The font used on the bar [default: DejaVu Sans Mono]
    -h, --height <height>        The height of the bar (in pixels) [default: 22]
    -t, --terminal <terminal>    The terminal to use when launching applications that require a terminal [default: i3-
                                 sensible-terminal]
```

### Installing

[This application is available on the AUR](https://aur.archlinux.org/packages/rlaunch/)
```
$ git clone https://aur.archlinux.org/rlaunch.git
$ cd rlaunch
$ makepkg -si
```

### Compiling from source
You will need `cargo` for this.
```
$ git clone https://github.com/PonasKovas/rlaunch.git
$ cd rlaunch
$ cargo build --release
```
After running these commands, the compiled binary will be `./target/release/rlaunch`

## Contributing

Feel free to make pull requests and issues, I will try to respond asap.

## Authors

* [PonasKovas](https://github.com/PonasKovas)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details
