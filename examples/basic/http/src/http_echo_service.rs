use allora::{service, Exchange, Message, Result, Service};
use async_trait::async_trait;

#[derive(Debug)]
pub struct HttpEcho;

#[service(name = "http_echo")]
impl HttpEcho {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Service for HttpEcho {
    async fn process(&self, ex: &mut Exchange) -> Result<()> {
        let in_body = ex.in_msg.body_text().unwrap_or("").trim();
        if !in_body.is_empty() {
            let reply = format!("Hello {in_body}!");
            ex.out_msg = Some(Message::from_text(reply.clone()));
        }
        Ok(())
    }
}
