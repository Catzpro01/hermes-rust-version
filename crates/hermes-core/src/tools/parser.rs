use super::{ToolCall, ToolError, ToolResponse};
use quick_xml::{events::Event, Reader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolEvent {
    Call(ToolCall),
    Response(ToolResponse),
}
pub fn parse_tool_events(input: &str) -> Result<Vec<ToolEvent>, ToolError> {
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut out = Vec::new();
    let mut current: Option<(String, Option<String>)> = None;
    let mut text = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if name == "tool_call" || name == "tool_response" {
                    let id = e
                        .attributes()
                        .flatten()
                        .find(|a| a.key.as_ref() == b"id")
                        .and_then(|a| String::from_utf8(a.value.into_owned()).ok());
                    current = Some((name, id));
                    text.clear();
                }
            }
            Ok(Event::Text(e)) => {
                if current.is_some() {
                    text.push_str(
                        &e.unescape()
                            .map_err(|e| ToolError::InvalidXml(e.to_string()))?,
                    );
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if let Some((kind, id)) = current.take() {
                    if kind == name {
                        if kind == "tool_call" {
                            let (tool, args) = text
                                .split_once(':')
                                .map(|(a, b)| (a.trim(), b.trim()))
                                .unwrap_or((text.trim(), ""));
                            out.push(ToolEvent::Call(ToolCall {
                                id,
                                name: tool.to_owned(),
                                arguments: args.to_owned(),
                            }));
                        } else {
                            out.push(ToolEvent::Response(ToolResponse {
                                id,
                                name: String::new(),
                                content: text.clone(),
                                success: true,
                            }));
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(ToolError::InvalidXml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_call_and_response() {
        let x=parse_tool_events("<tools><tool_call id=\"1\">shell: echo hi</tool_call><tool_response>ok</tool_response></tools>").unwrap();
        assert_eq!(x.len(), 2);
        assert!(matches!(&x[0],ToolEvent::Call(c) if c.name=="shell"));
    }
}
