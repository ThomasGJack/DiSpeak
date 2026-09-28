use dispeak_shared::events::{ClientEvent, ServerEvent, ServerEventEnvelope};
use serde_json::Value;

pub fn parse_client_event(action: &str) -> ClientEvent {
    serde_json::from_str(&format!("\"{}\"", action)).unwrap_or(ClientEvent::Unknown)
}

pub fn server_envelope(event: ServerEvent, data: Value) -> ServerEventEnvelope {
    ServerEventEnvelope::new(event, data)
}

