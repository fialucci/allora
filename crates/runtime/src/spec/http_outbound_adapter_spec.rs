use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct HttpOutboundAdapterSpecRootV1 {
    pub version: u32,
    #[serde(rename = "http-outbound-adapter")]
    pub http_outbound_adapter: HttpOutboundAdapterSpecBlock,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HttpOutboundAdapterSpecBlock {
    pub id: Option<String>,
    /// Full target URL (must include scheme — `http://` or `https://`).
    ///
    /// Replaces the legacy `host` + `port` + `base-path` triple as of
    /// 0.0.9. Parsing happens in the YAML parser; invalid URLs surface
    /// as `Error::Serialization` rather than panicking at dispatch time.
    pub url: String,
    pub method: Option<String>,
    #[serde(rename = "use-out-msg")]
    pub use_out_msg: Option<bool>,
    /// Channel-driven dispatch: when present, the runtime subscribes this
    /// adapter to the named inbound channel and dispatches each arriving
    /// exchange. When absent, the adapter is built but not auto-wired —
    /// application code can still invoke `.dispatch(&exchange)` directly
    /// (the legacy http-outbound example pattern).
    pub from: Option<String>,
    /// Outbound channel for the post-dispatch exchange. Ignored when
    /// `from` is absent. When `from` is set but `to` is `None`, the
    /// adapter is fire-and-forget: dispatch happens, the result is
    /// logged, the message is dropped.
    pub to: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HttpOutboundAdapterSpec(HttpOutboundAdapterSpecBlock);

impl HttpOutboundAdapterSpec {
    pub(crate) fn from_block(b: HttpOutboundAdapterSpecBlock) -> Self {
        Self(b)
    }
    /// Programmatic constructor.
    ///
    /// `url` must include the scheme (`http://` or `https://`). It is not
    /// validated here — validation happens when the spec is constructed
    /// via the YAML parser, or when the adapter is built. Callers using
    /// this constructor directly are expected to provide a parseable URL.
    pub fn new(url: &str, method: Option<&str>, id: Option<&str>, use_out_msg: bool) -> Self {
        let blk = HttpOutboundAdapterSpecBlock {
            id: id.map(|s| s.to_string()),
            url: url.to_string(),
            method: method.map(|m| m.to_string()),
            use_out_msg: Some(use_out_msg),
            from: None,
            to: None,
        };
        Self(blk)
    }
    /// Set the inbound channel name the runtime will subscribe this
    /// adapter to. Returns `self` for chaining; intended for fluent test
    /// fixture construction.
    pub fn with_from(mut self, from: impl Into<String>) -> Self {
        self.0.from = Some(from.into());
        self
    }
    /// Set the outbound channel name for the post-dispatch exchange.
    pub fn with_to(mut self, to: impl Into<String>) -> Self {
        self.0.to = Some(to.into());
        self
    }
    pub fn with_id(id: &str, url: &str, method: Option<&str>, use_out_msg: bool) -> Self {
        Self::new(url, method, Some(id), use_out_msg)
    }
    pub fn id(&self) -> Option<&str> {
        self.0.id.as_deref()
    }
    /// Target URL as configured (raw string). Includes scheme.
    pub fn url(&self) -> &str {
        &self.0.url
    }
    pub fn method(&self) -> Option<&str> {
        self.0.method.as_deref()
    }
    pub fn use_out_msg(&self) -> bool {
        self.0.use_out_msg.unwrap_or(true)
    }
    /// Inbound channel name for channel-driven dispatch, if set.
    pub fn from(&self) -> Option<&str> {
        self.0.from.as_deref()
    }
    /// Outbound channel name for the post-dispatch exchange, if set.
    pub fn to(&self) -> Option<&str> {
        self.0.to.as_deref()
    }
}
