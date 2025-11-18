use allora::{error::Result, message::Payload, service, Exchange, Service};

#[derive(Clone, Debug)]
pub struct HelloService;

#[service(name = "hello_world")]
impl HelloService {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Service for HelloService {
    async fn process(&self, exchange: &mut Exchange) -> Result<()> {
        if let Some(name) = exchange.in_msg.body_text() {
            exchange.in_msg.payload = Payload::Text(format!("Hello {name}!"));
        }
        Ok(())
    }
}
