use allora::{error::Result, service, Exchange, Service};

#[derive(Debug)]
struct DefaultSvc;

#[service]
impl DefaultSvc {
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(feature = "async", async_trait::async_trait)]
impl Service for DefaultSvc {
    #[cfg(feature = "async")]
    async fn process(&self, _ex: &mut Exchange) -> Result<()> {
        Ok(())
    }
    #[cfg(not(feature = "async"))]
    fn process(&self, _ex: &mut Exchange) -> Result<()> {
        Ok(())
    }
}

fn main() {}
