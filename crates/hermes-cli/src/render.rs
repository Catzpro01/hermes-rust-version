use futures::StreamExt;
use hermes_core::conversation::Event;
use hermes_core::provider::EventStream;
use std::io::{self, Write};

#[allow(dead_code)]
pub async fn render_stream(mut events: EventStream) -> anyhow::Result<String> {
    let mut full = String::new();
    while let Some(event) = events.next().await {
        match event? {
            Event::Started => {}
            Event::Chunk(text) => {
                print!("{text}");
                io::stdout().flush()?;
                full.push_str(&text);
            }
            Event::Done => println!(),
            Event::Error(message) => eprintln!("\n⚠ {message}"),
            Event::ToolCall(call) => eprintln!(
                "
[tool_call {}]",
                call.name
            ),
        }
    }
    Ok(full)
}
