use super::ProviderError;
use crate::conversation::Event;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ChunkResponse {
    choices: Vec<Choice>,
}
#[derive(Debug, Deserialize)]
struct Choice {
    delta: Delta,
    finish_reason: Option<String>,
}
#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
}

/// Maps one OpenAI-compatible SSE data payload to a runtime event.
pub fn parse_data(data: &str) -> Result<Event, ProviderError> {
    if data.trim() == "[DONE]" {
        return Ok(Event::Done);
    }
    let parsed: ChunkResponse = serde_json::from_str(data)
        .map_err(|e| ProviderError::Message(format!("malformed SSE payload: {e}")))?;
    let choice = parsed
        .choices
        .first()
        .ok_or_else(|| ProviderError::Message("SSE payload has no choices".into()))?;
    if choice.finish_reason.is_some() && choice.delta.content.is_none() {
        return Ok(Event::Done);
    }
    Ok(Event::Chunk(
        choice.delta.content.clone().unwrap_or_default(),
    ))
}

/// Parses complete `data:` lines from a byte chunk. The returned remainder is an incomplete line.
pub fn parse_chunk(bytes: &[u8], remainder: &mut Vec<u8>) -> Result<Vec<Event>, ProviderError> {
    remainder.extend_from_slice(bytes);
    let mut events = Vec::new();
    while let Some(pos) = remainder.iter().position(|b| *b == b'\n') {
        let line: Vec<u8> = remainder.drain(..=pos).collect();
        let line = String::from_utf8_lossy(&line).trim().to_owned();
        if let Some(data) = line.strip_prefix("data:") {
            if !data.trim().is_empty() {
                events.push(parse_data(data.trim())?);
            }
        }
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_chunk_and_done() {
        let mut remainder = Vec::new();
        let bytes = b"data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\ndata: [DONE]\n\n";
        assert_eq!(
            parse_chunk(bytes, &mut remainder).unwrap(),
            vec![Event::Chunk("hi".into()), Event::Done]
        );
        assert!(remainder.is_empty());
    }
    #[test]
    fn preserves_partial_sse_line() {
        let mut remainder = Vec::new();
        assert!(parse_chunk(b"data: {\"choices\":[", &mut remainder)
            .unwrap()
            .is_empty());
        let events = parse_chunk(b"{\"delta\":{\"content\":\"x\"}}]}\n", &mut remainder).unwrap();
        assert_eq!(events, vec![Event::Chunk("x".into())]);
    }
    #[test]
    fn rejects_malformed_payload() {
        assert!(parse_data("not-json").is_err());
    }
}
