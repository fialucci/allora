use allora::{error::Result, service, Exchange, Service};

#[derive(Debug)]
struct NamedSvc;
#[service(name = "named")]
impl NamedSvc {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait::async_trait]
impl Service for NamedSvc {
    async fn process(&self, _ex: &mut Exchange) -> Result<()> {
        Ok(())
    }
}

fn main() {}
