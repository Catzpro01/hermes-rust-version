//! Evidence adapter: invoke the real banner component with explicit dummy data.
//! No UI text or formatting is reconstructed here; not a new CLI feature.
#[allow(dead_code)]
#[path = "../src/tui/art.rs"]
mod art;
#[path = "../src/tui/theme.rs"]
mod theme;
#[allow(dead_code)]
#[path = "../src/tui/welcome.rs"]
mod welcome;

use std::io::Write;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let count: usize = args[1].parse().expect("tool count");
    let width: u16 = args[2].parse().expect("terminal width");
    let info = welcome::BannerInfo {
        model: Some("parity-fixture".to_owned()),
        cwd: "/fixture/demo".to_owned(),
        tools: ["list_dir", "read_file", "write_file"]
            .iter()
            .take(count)
            .map(|name| (*name).to_owned())
            .collect(),
        ..Default::default()
    };
    welcome::write_banner(
        &mut std::io::stdout(),
        &theme::HermesTheme::dark_canonical(),
        width,
        &info,
        theme::ColorDepth::Truecolor,
    )?;
    // Match Console.print and the real REPL caller, which terminate the panel line.
    writeln!(std::io::stdout())
}
