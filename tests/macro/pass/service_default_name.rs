use allora::{error::Result, service, Exchange, Service};

#[derive(Debug)]
struct DefaultSvc;

#[service]
impl DefaultSvc {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Service for DefaultSvc {
    async fn process(&self, _ex: &mut Exchange) -> Result<()> {
        Ok(())
    }
}

fn main() {}
