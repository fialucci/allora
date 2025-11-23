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
    pub host: String,
    pub port: u16,
    #[serde(rename = "base-path")]
    pub base_path: Option<String>,
    pub path: Option<String>,
    pub method: Option<String>,
    #[serde(rename = "use-out-msg")]
    pub use_out_msg: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct HttpOutboundAdapterSpec(HttpOutboundAdapterSpecBlock);

impl HttpOutboundAdapterSpec {
    pub(crate) fn from_block(b: HttpOutboundAdapterSpecBlock) -> Self {
        Self(b)
    }
    pub fn new(
        host: &str,
        port: u16,
        base_path: &str,
        path: Option<&str>,
        method: Option<&str>,
        id: Option<&str>,
        use_out_msg: bool,
    ) -> Self {
        let blk = HttpOutboundAdapterSpecBlock {
            id: id.map(|s| s.to_string()),
            host: host.to_string(),
            port,
            base_path: Some(base_path.to_string()),
            path: path.map(|p| p.to_string()),
            method: method.map(|m| m.to_string()),
            use_out_msg: Some(use_out_msg),
        };
        Self(blk)
    }
    pub fn with_id(
        id: &str,
        host: &str,
        port: u16,
        base_path: &str,
        path: Option<&str>,
        method: Option<&str>,
        use_out_msg: bool,
    ) -> Self {
        Self::new(host, port, base_path, path, method, Some(id), use_out_msg)
    }
    pub fn id(&self) -> Option<&str> {
        self.0.id.as_deref()
    }
    pub fn host(&self) -> &str {
        &self.0.host
    }
    pub fn port(&self) -> u16 {
        self.0.port
    }
    pub fn base_path(&self) -> &str {
        self.0.base_path.as_deref().unwrap_or("/")
    }
    pub fn path(&self) -> Option<&str> {
        self.0.path.as_deref()
    }
    pub fn method(&self) -> Option<&str> {
        self.0.method.as_deref()
    }
    pub fn use_out_msg(&self) -> bool {
        self.0.use_out_msg.unwrap_or(true)
    }
}
