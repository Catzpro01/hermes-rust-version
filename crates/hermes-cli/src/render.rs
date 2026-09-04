use hermes_core::conversation::Event;
use std::io::{self, Write};

pub fn render_events(events: &[Event]) -> anyhow::Result<()> {
    for event in events {
        match event {
            Event::Started => {}
            Event::Chunk(text) => print!("{text}"),
            Event::Done => println!(),
            Event::Error(message) => eprintln!("\n⚠ {message}"),
        }
        io::stdout().flush()?;
    }
    Ok(())
}
