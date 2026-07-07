//! Startup banner (stderr). Purely cosmetic — this CLI only ever runs in CI
//! logs, so make the greeting worth the pixels. Honors `NO_COLOR`.

// "SYLLABUS" — Standard figlet font.
const WORDMARK: &[&str] = &[
    r" ____  _   _ _     _        _    ____  _   _ ____  ",
    r"/ ___|| | | | |   | |      / \  | __ )| | | / ___| ",
    r"\___ \| | | | |   | |     / _ \ |  _ \| | | \___ \ ",
    r" ___) | |_| | |___| |___ / ___ \| |_) | |_| |___) |",
    r"|____/ \___/|_____|_____/_/   \_\____/ \___/|____/ ",
];

// A cyan→blue gradient down the five wordmark rows (256-color; Actions renders it).
const SHADES: [&str; 5] = ["38;5;51", "38;5;45", "38;5;39", "38;5;33", "38;5;27"];

/// Print the startup banner to stderr (a no-op's worth of bytes, all vibes).
pub fn print() {
    let color = std::env::var_os("NO_COLOR").is_none();
    let paint = |code: &str, s: &str| {
        if color {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_owned()
        }
    };
    eprintln!();
    for (line, shade) in WORDMARK.iter().zip(SHADES) {
        eprintln!("  {}", paint(shade, line));
    }
    eprintln!(
        "  {}   {}  {}",
        paint("1;96", "🐟 逆引きシラバス"),
        paint("2", &format!("v{}", env!("CARGO_PKG_VERSION"))),
        paint(
            "2",
            "· reverse-lookup syllabus toolkit · KULAS crawler + converter"
        ),
    );
    eprintln!();
}
