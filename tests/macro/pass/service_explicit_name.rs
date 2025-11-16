use allora::{error::Result, service, Exchange, Service};
#[derive(Debug)]
struct NamedSvc;
#[service(name = "named")]
impl NamedSvc {
    pub fn new() -> Self {
        Self
    }
}
#[cfg_attr(feature = "async", async_trait::async_trait)]
impl Service for NamedSvc {
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
