use meteroid_store::domain::outbox_event::CustomerCreatedEvent;
use rdkafka::message::{BorrowedHeaders, BorrowedMessage, Headers};
use rdkafka::Message;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug)]
pub struct OutboxEvent {
    pub id: Uuid,
    pub tenant_id: Uuid,
    #[allow(dead_code)]
    pub aggregate_id: String,
    pub event_type: EventType,
}

#[derive(Debug)]
pub enum EventType {
    CustomerCreated(Box<CustomerCreatedEvent>),
    InvoiceFinalized,
    InvoicePdfRequested,
}

impl EventType {
    /// This function falls back to None in case of parsing error
    /// todo return Result<Option<EventType>, Error>
    pub fn from_kafka_message(m: &BorrowedMessage<'_>) -> Option<Self> {
        let headers = m.headers()?;
        let event_type = headers.get_as_string("event_type")?;

        match event_type.as_str() {
            "customer.created" => {
                let payload = extract_payload::<CustomerCreatedEvent>(m).ok()??;
                Some(Self::CustomerCreated(Box::new(payload)))
            }
            "invoice.finalized" => Some(Self::InvoiceFinalized),
            "invoice.pdf.requested" => Some(Self::InvoicePdfRequested),
            _ => None,
        }
    }
}

/// This function falls back to None in case of parsing error
/// todo return Result<Option<OutboxEvent>, Error>
pub(crate) fn parse_outbox_event(m: &BorrowedMessage<'_>) -> Option<OutboxEvent> {
    let headers = m.headers()?;
    let id = headers.get_as_uuid("id")?;
    let tenant_id = headers.get_as_uuid("tenant_id")?;

    let aggregate_id: String = String::from_utf8(m.key()?.to_vec())
        .ok()?
        .trim_matches('"')
        .into();

    let event_type = EventType::from_kafka_message(m)?;

    Some(OutboxEvent {
        id,
        tenant_id,
        aggregate_id,
        event_type,
    })
}

fn extract_payload<P: for<'a> Deserialize<'a>>(
    m: &BorrowedMessage<'_>,
) -> Result<Option<P>, serde_json::Error> {
    if let Some(payload) = m.payload() {
        let parsed: Value = serde_json::from_slice(payload)?;
        let payload = &parsed["payload"];
        let parsed = serde_json::from_value(payload.clone())?;
        Ok(Some(parsed))
    } else {
        Ok(None)
    }
}

trait ParseableHeaders {
    fn get_as_string(&self, key: &str) -> Option<String>;
    fn get_as_uuid(&self, key: &str) -> Option<uuid::Uuid>;
}

impl ParseableHeaders for &BorrowedHeaders {
    fn get_as_string(&self, key: &str) -> Option<String> {
        let header_value = self
            .iter()
            .find_map(|x| if x.key == key { x.value } else { None })?;

        String::from_utf8(header_value.to_vec()).ok()
    }

    fn get_as_uuid(&self, key: &str) -> Option<uuid::Uuid> {
        self.get_as_string(key)
            .and_then(|header_value| uuid::Uuid::parse_str(&header_value).ok())
    }
}
