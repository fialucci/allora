//! Messaging core types: `Payload`, `Message`, and `Exchange`.
//!
//! # Overview
//! These types model the fundamental unit that flows through a Route / processors.
//! * `Payload` – concrete body representation (text, bytes, json, empty).
//! * `Message` – wraps a `Payload` plus string key/value headers (metadata).
//! * `Exchange` – carries an inbound `Message`, an optional outbound `Message`, and free-form properties used during routing.
//!
//! # IDs & Correlation
//! Every `Message` automatically gets a unique `message_id` header when constructed via `Message::new` / `Message::from_text`.
//! A `correlation_id` is NOT generated automatically—call `Message::ensure_correlation_id()` or `Exchange::correlation_id()` (or use the `CorrelationInitializer` processor / `Route::with_correlation`) to lazily create one when needed (aggregation, split/join, request/reply, etc.).
//!
//! # Headers vs Properties
//! * Headers live on the `Message` and are typically serialized/shared externally.
//! * Properties live on the `Exchange` and are intended for transient, internal routing state.
//! Choose headers when downstream systems or processors need the metadata; choose properties for ephemeral routing hints.
//!
//! # Thread Safety / Mutation
//! These types are plain structs; mutation requires &mut access. If sharing between threads, wrap in synchronization primitives (e.g. Arc<Mutex<_>>). The library leaves concurrency control to callers for flexibility.
//!
//! # Serialization
//! All types derive `Serialize`/`Deserialize` unconditionally; JSON bodies use `serde_json::Value`.
//!
//! # Examples
//! Basic creation:
//! ```no_run
//! use allora_core::message::{Exchange, Message};
//! let mut ex = Exchange::new(Message::from_text("ping"));
//! assert_eq!(ex.in_msg.body_text(), Some("ping"));
//! ```
//!
//! With bytes payload:
//! ```no_run
//! use allora_core::message::{Message, Payload};
//! let msg = Message::from_text("hello");
//! assert_eq!(msg.body_text(), Some("hello"));
//! ```
//!
//! Adding and overriding headers:
//! ```no_run
//! use allora_core::message::Message;
//! let msg = Message::from_text("demo");
//! assert!(msg.header("missing").is_none());
//! ```
//! [`CorrelationInitializer`]: crate::patterns::correlation_initializer::CorrelationInitializer
//! [`Route::with_correlation`]: crate::route::Route::with_correlation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the payload of a message, supporting text, bytes, JSON, or empty.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Payload {
    /// UTF-8 text payload.
    Text(String),
    /// Raw bytes payload.
    Bytes(Vec<u8>),
    /// JSON value payload (only when serde feature enabled).
    Json(serde_json::Value),
    /// Empty payload.
    Empty,
}

impl Default for Payload {
    /// Returns an empty payload.
    fn default() -> Self {
        Payload::Empty
    }
}

impl Payload {
    /// Returns the payload as text, if available.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Payload::Text(s) => Some(s),
            _ => None,
        }
    }
    /// Returns the payload as bytes, if available.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Payload::Bytes(b) => Some(b),
            _ => None,
        }
    }
}

/// A message containing a payload and headers (metadata).
///
/// # Automatic Message ID
/// On construction a UUID v4 is inserted under the `message_id` header. While you *can*
/// override this with `set_header`, it is discouraged—other infrastructure (logging, tracing)
/// may rely on uniqueness.
///
/// # Correlation ID
/// Not created automatically to avoid overhead if unused. Call [`Message::ensure_correlation_id`]
/// when a processor needs it. Subsequent calls return the same stable value.
///
/// # Header Semantics
/// All headers are simple UTF-8 strings. Keep values small; store large / complex structures
/// in the payload or properties instead.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Message {
    /// The message payload.
    pub payload: Payload,
    /// Arbitrary string headers for metadata.
    pub headers: HashMap<String, String>,
}

impl Message {
    /// Creates a new message with the given payload.
    pub fn new(payload: Payload) -> Self {
        let mut m = Self {
            payload,
            headers: HashMap::new(),
        };
        // Automatically assign a unique message id header.
        let mid = uuid::Uuid::new_v4().to_string();
        m.headers.insert("message_id".to_string(), mid);
        m
    }
    /// Creates a new text message.
    pub fn from_text<T: Into<String>>(s: T) -> Self {
        Self::new(Payload::Text(s.into()))
    }
    /// Returns the message body as text, if available.
    pub fn body_text(&self) -> Option<&str> {
        self.payload.as_text()
    }
    /// Returns the value of a header, if present.
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(|s| s.as_str())
    }
    /// Sets a header key-value pair.
    pub fn set_header<K: Into<String>, V: Into<String>>(&mut self, k: K, v: V) {
        self.headers.insert(k.into(), v.into());
    }
    /// Ensures a correlation id header exists; returns its value.
    ///
    /// If a `correlation_id` header is present it is returned unchanged; otherwise a new
    /// UUID v4 is generated, inserted, and then returned.
    ///
    /// # Use Cases
    /// * Aggregator / Splitter patterns grouping related messages.
    /// * Request/Reply correlation.
    /// * Distributed tracing boundary (can pair with trace/span ids).
    ///
    /// Prefer calling once early (or using `CorrelationInitializer`) rather than sprinkling
    /// calls across processors.
    pub fn ensure_correlation_id(&mut self) -> &str {
        if !self.headers.contains_key("correlation_id") {
            let cid = uuid::Uuid::new_v4().to_string();
            self.headers.insert("correlation_id".to_string(), cid);
        }
        self.headers.get("correlation_id").unwrap()
    }
}

/// An exchange wraps an inbound and outbound message, plus routing properties.
///
/// # Inbound vs Outbound
/// * `in_msg` – original message entering the pipeline.
/// * `out_msg` – optional transformed result set by processors (e.g. formatter, enricher).
///
/// # Properties
/// Free-form key/value pairs for internal routing state (e.g. retry count, timing info).
/// Not automatically serialized; use headers for externally visible metadata.
///
/// # Correlation Helper
/// `Exchange::correlation_id` provides a convenience wrapper around `in_msg.ensure_correlation_id()`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Exchange {
    /// The inbound message.
    pub in_msg: Message,
    /// The outbound message, if set.
    pub out_msg: Option<Message>,
    /// Arbitrary string properties for routing or context.
    pub properties: HashMap<String, String>,
}

impl Exchange {
    /// Creates a new exchange with the given inbound message.
    pub fn new(msg: Message) -> Self {
        Self {
            in_msg: msg,
            out_msg: None,
            properties: HashMap::new(),
        }
    }
    /// Sets a property key-value pair.
    pub fn set_property<K: Into<String>, V: Into<String>>(&mut self, k: K, v: V) {
        self.properties.insert(k.into(), v.into());
    }
    /// Returns the value of a property, if present.
    pub fn property(&self, k: &str) -> Option<&str> {
        self.properties.get(k).map(|s| s.as_str())
    }
    /// Ensures the inbound message has a correlation id, returning it.
    /// Equivalent to `self.in_msg.ensure_correlation_id()`; provided for ergonomic chaining
    /// in routing code.
    pub fn correlation_id(&mut self) -> &str {
        self.in_msg.ensure_correlation_id()
    }
}

/// Alias for Payload type.
pub use Payload as RawPayload;

