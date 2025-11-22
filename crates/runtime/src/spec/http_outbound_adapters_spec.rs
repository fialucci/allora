use crate::spec::HttpOutboundAdapterSpec;

#[derive(Debug, Clone)]
pub struct HttpOutboundAdaptersSpec {
    version: u32,
    adapters: Vec<HttpOutboundAdapterSpec>,
}

impl HttpOutboundAdaptersSpec {
    pub fn new(version: u32) -> Self {
        Self {
            version,
            adapters: Vec::new(),
        }
    }
    pub fn add(mut self, a: HttpOutboundAdapterSpec) -> Self {
        self.adapters.push(a);
        self
    }
    pub fn push(&mut self, a: HttpOutboundAdapterSpec) {
        self.adapters.push(a);
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn adapters(&self) -> &[HttpOutboundAdapterSpec] {
        &self.adapters
    }
    pub fn into_adapters(self) -> Vec<HttpOutboundAdapterSpec> {
        self.adapters
    }
}
